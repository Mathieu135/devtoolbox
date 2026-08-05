#!/usr/bin/env bash
set -euo pipefail

repository="Mathieu135/devtoolbox"
version="${DEVTOOLBOX_VERSION:-latest}"
install_dir="${DEVTOOLBOX_INSTALL_DIR:-${HOME}/.local/bin}"

case "$(uname -s)" in
  Linux) platform="linux" ;;
  Darwin) platform="macos" ;;
  *)
    echo "Système non supporté : $(uname -s)" >&2
    exit 1
    ;;
esac

case "$(uname -m)" in
  x86_64|amd64) architecture="x86_64" ;;
  arm64|aarch64) architecture="aarch64" ;;
  *)
    echo "Architecture non supportée : $(uname -m)" >&2
    exit 1
    ;;
esac

archive="toolbox-${platform}-${architecture}.tar.gz"
if [[ "${version}" == "latest" ]]; then
  release_path="latest/download"
else
  release_path="download/${version}"
fi
download_url="https://github.com/${repository}/releases/${release_path}"

temporary_dir="$(mktemp -d)"
cleanup() {
  rm -rf -- "${temporary_dir}"
}
trap cleanup EXIT

curl -fsSL --output "${temporary_dir}/${archive}" "${download_url}/${archive}"
curl -fsSL --output "${temporary_dir}/${archive}.sha256" "${download_url}/${archive}.sha256"

(
  cd "${temporary_dir}"
  shasum -a 256 -c "${archive}.sha256"
  tar -xzf "${archive}"
)

mkdir -p "${install_dir}"
install -m 0755 "${temporary_dir}/toolbox" "${install_dir}/toolbox"
echo "toolbox installé dans ${install_dir}/toolbox"
case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *) echo "Ajoutez-le au PATH : export PATH=\"${install_dir}:\$PATH\"" ;;
esac
