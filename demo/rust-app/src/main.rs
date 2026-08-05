fn main() {
    let rabbitmq_url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "RabbitMQ n'est pas encore configuré".to_owned());
    println!("Application de démonstration : {rabbitmq_url}");
}
