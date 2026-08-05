use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use futures_lite::stream::StreamExt;
use lapin::{
    BasicProperties, Channel, Connection, ConnectionProperties,
    options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions},
    types::FieldTable,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

const QUEUE_NAME: &str = "devtoolbox.demo.messages";
const REDIS_COUNTER_KEY: &str = "devtoolbox:demo:counter";
const DEFAULT_RABBITMQ_URL: &str = "amqp://toolbox:toolbox-local-only@127.0.0.1:5672/%2f";
const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379/";

#[derive(Clone)]
struct AppState {
    publisher: Channel,
    _rabbitmq_connection: Arc<Connection>,
    redis: redis::Client,
    consumed: Arc<RwLock<Vec<QueueMessage>>>,
    sequence: Arc<AtomicU64>,
}

#[derive(Clone, Deserialize, Serialize)]
struct QueueMessage {
    id: String,
    text: String,
    origin: String,
    published_at_ms: u64,
}

#[derive(Deserialize)]
struct PublishRequest {
    text: String,
}

#[derive(Serialize)]
struct PublishResponse {
    queued: bool,
    message_id: String,
}

#[derive(Serialize)]
struct CounterResponse {
    value: i64,
}

#[derive(Serialize)]
struct CheckResponse {
    service: &'static str,
    ok: bool,
    latency_ms: u64,
    detail: &'static str,
}

#[derive(Serialize)]
struct StateResponse {
    rabbitmq: ServiceState,
    redis: ServiceState,
    redis_counter: i64,
    consumed_messages: Vec<QueueMessage>,
}

#[derive(Serialize)]
struct ServiceState {
    online: bool,
    detail: String,
}

struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn internal(error: impl std::fmt::Display) -> Self {
        eprintln!("Erreur API interne : {error}");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "une erreur interne est survenue".into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: message.into(),
        }
    }

    fn unavailable(service: &'static str, error: impl std::fmt::Display) -> Self {
        eprintln!("Check {service} en échec : {error}");
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "service_unavailable",
            message: format!("{service} ne répond pas correctement"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "code": self.code,
                "error": self.message,
            })),
        )
            .into_response()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let rabbitmq_url = env::var("RABBITMQ_URL").unwrap_or_else(|_| DEFAULT_RABBITMQ_URL.into());
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.into());
    let port = env::var("APP_PORT").unwrap_or_else(|_| "3000".into());

    let connection = Arc::new(
        Connection::connect(
            &rabbitmq_url,
            ConnectionProperties::default().with_connection_name("devtoolbox-web-demo".into()),
        )
        .await
        .context("connexion à RabbitMQ ; lancez d'abord `toolbox up`")?,
    );
    let publisher = connection.create_channel().await?;
    publisher
        .queue_declare(
            QUEUE_NAME.into(),
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await?;

    let consumed = Arc::new(RwLock::new(Vec::new()));
    start_consumer(&connection, Arc::clone(&consumed)).await?;

    let redis = redis::Client::open(redis_url).context("configuration du client Redis")?;
    let mut redis_connection = redis
        .get_multiplexed_async_connection()
        .await
        .context("connexion à Redis ; lancez d'abord `toolbox up`")?;
    let _: String = redis::cmd("PING")
        .query_async(&mut redis_connection)
        .await?;

    let state = AppState {
        publisher,
        _rabbitmq_connection: connection,
        redis,
        consumed,
        sequence: Arc::new(AtomicU64::new(1)),
    };

    let startup = new_message(&state, "Le site vient de démarrer", "automatique");
    publish(&state.publisher, &startup).await?;

    let app = Router::new()
        .route("/", get(index))
        .route("/api/state", get(current_state))
        .route("/api/messages", post(send_message))
        .route("/api/redis/increment", post(increment_counter))
        .route("/api/checks/rabbitmq", post(check_rabbitmq))
        .route("/api/checks/redis", post(check_redis))
        .with_state(state);

    let address = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("DevToolbox Lab disponible sur http://{address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn start_consumer(
    connection: &Connection,
    consumed: Arc<RwLock<Vec<QueueMessage>>>,
) -> Result<()> {
    let channel = connection.create_channel().await?;
    let mut consumer = channel
        .basic_consume(
            QUEUE_NAME.into(),
            "devtoolbox-web-demo".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tokio::spawn(async move {
        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(delivery) => {
                    let message = serde_json::from_slice::<QueueMessage>(&delivery.data);
                    match delivery.ack(BasicAckOptions::default()).await {
                        Ok(true) => {
                            if let Ok(message) = message {
                                let mut messages = consumed.write().await;
                                messages.insert(0, message);
                                messages.truncate(20);
                            }
                        }
                        Ok(false) => eprintln!("ACK RabbitMQ ignoré : acker déjà utilisé"),
                        Err(error) => eprintln!("ACK RabbitMQ impossible : {error}"),
                    }
                }
                Err(error) => eprintln!("Consommation RabbitMQ interrompue : {error}"),
            }
        }
    });
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

async fn current_state(State(state): State<AppState>) -> Json<StateResponse> {
    let (redis_state, redis_counter) = match state.redis.get_multiplexed_async_connection().await {
        Ok(mut connection) => {
            let value: redis::RedisResult<Option<i64>> = connection.get(REDIS_COUNTER_KEY).await;
            match value {
                Ok(value) => (
                    ServiceState {
                        online: true,
                        detail: "lecture GET réussie".into(),
                    },
                    value.unwrap_or(0),
                ),
                Err(error) => (
                    {
                        eprintln!("Lecture Redis impossible : {error}");
                        ServiceState {
                            online: false,
                            detail: "lecture GET échouée".into(),
                        }
                    },
                    0,
                ),
            }
        }
        Err(error) => {
            eprintln!("Connexion Redis impossible : {error}");
            (
                ServiceState {
                    online: false,
                    detail: "connexion Redis échouée".into(),
                },
                0,
            )
        }
    };

    Json(StateResponse {
        rabbitmq: ServiceState {
            online: true,
            detail: format!("queue {QUEUE_NAME} consommée et ACKée"),
        },
        redis: redis_state,
        redis_counter,
        consumed_messages: state.consumed.read().await.clone(),
    })
}

async fn send_message(
    State(state): State<AppState>,
    Json(request): Json<PublishRequest>,
) -> Result<Json<PublishResponse>, ApiError> {
    let text = request.text.trim();
    if text.is_empty() {
        return Err(ApiError::bad_request("le message ne peut pas être vide"));
    }
    if text.chars().count() > 240 {
        return Err(ApiError::bad_request(
            "le message est limité à 240 caractères",
        ));
    }

    let message = new_message(&state, text, "bouton du site");
    publish(&state.publisher, &message)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(PublishResponse {
        queued: true,
        message_id: message.id,
    }))
}

async fn increment_counter(
    State(state): State<AppState>,
) -> Result<Json<CounterResponse>, ApiError> {
    let mut connection = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(ApiError::internal)?;
    let value: i64 = connection
        .incr(REDIS_COUNTER_KEY, 1_i64)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(CounterResponse { value }))
}

async fn check_rabbitmq(State(state): State<AppState>) -> Result<Json<CheckResponse>, ApiError> {
    let started = Instant::now();
    let message = new_message(&state, "Sonde de diagnostic RabbitMQ", "bouton check");
    publish(&state.publisher, &message)
        .await
        .map_err(|error| ApiError::unavailable("RabbitMQ", error))?;
    wait_until_consumed(&state, &message.id)
        .await
        .map_err(|error| ApiError::unavailable("RabbitMQ", error))?;

    Ok(Json(CheckResponse {
        service: "rabbitmq",
        ok: true,
        latency_ms: elapsed_ms(started),
        detail: "message publié, consommé et ACKé",
    }))
}

async fn check_redis(State(state): State<AppState>) -> Result<Json<CheckResponse>, ApiError> {
    let started = Instant::now();
    let mut connection = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| ApiError::unavailable("Redis", error))?;

    let pong: String = redis::cmd("PING")
        .query_async(&mut connection)
        .await
        .map_err(|error| ApiError::unavailable("Redis", error))?;
    if pong != "PONG" {
        return Err(ApiError::unavailable(
            "Redis",
            format!("réponse PING inattendue : {pong}"),
        ));
    }

    let probe_id = state.sequence.fetch_add(1, Ordering::Relaxed);
    let probe_key = format!("devtoolbox:demo:check:{probe_id}");
    let _: String = redis::cmd("SET")
        .arg(&probe_key)
        .arg("roundtrip-ok")
        .arg("EX")
        .arg(5)
        .query_async(&mut connection)
        .await
        .map_err(|error| ApiError::unavailable("Redis", error))?;
    let value: Option<String> = redis::cmd("GET")
        .arg(&probe_key)
        .query_async(&mut connection)
        .await
        .map_err(|error| ApiError::unavailable("Redis", error))?;
    if value.as_deref() != Some("roundtrip-ok") {
        return Err(ApiError::unavailable(
            "Redis",
            "la valeur relue ne correspond pas à la sonde",
        ));
    }
    let _: i64 = redis::cmd("DEL")
        .arg(&probe_key)
        .query_async(&mut connection)
        .await
        .map_err(|error| ApiError::unavailable("Redis", error))?;

    Ok(Json(CheckResponse {
        service: "redis",
        ok: true,
        latency_ms: elapsed_ms(started),
        detail: "PING puis SET/GET temporaire réussis",
    }))
}

async fn wait_until_consumed(state: &AppState, message_id: &str) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if state
            .consumed
            .read()
            .await
            .iter()
            .any(|message| message.id == message_id)
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            anyhow::bail!("la sonde n'a pas été consommée dans les 3 secondes")
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

async fn publish(channel: &Channel, message: &QueueMessage) -> Result<()> {
    let payload = serde_json::to_vec(message)?;
    channel
        .basic_publish(
            "".into(),
            QUEUE_NAME.into(),
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default()
                .with_content_type("application/json".into())
                .with_delivery_mode(2),
        )
        .await?
        .await?;
    Ok(())
}

fn new_message(
    state: &AppState,
    text: impl Into<String>,
    origin: impl Into<String>,
) -> QueueMessage {
    let timestamp = now_ms();
    let sequence = state.sequence.fetch_add(1, Ordering::Relaxed);
    QueueMessage {
        id: format!("msg-{timestamp}-{sequence}"),
        text: text.into(),
        origin: origin.into(),
        published_at_ms: timestamp,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
