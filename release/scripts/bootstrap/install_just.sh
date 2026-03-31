#!/usr/bin/env bash

set -euo pipefail
IFS=$'\n\t'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DRY_RUN="${DRY_RUN:-0}"

log() {
  local level="$1"
  shift
  printf '[%s] [%s] %s\n' "$(date +'%Y-%m-%d %H:%M:%S')" "$level" "$*" >&2
}

info() {
  log INFO "$@"
}

warn() {
  log WARN "$@"
}

die() {
  log ERROR "$@"
  exit 1
}

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

run_cmd() {
  local cmd="$1"
  shift || true

  if [[ "${DRY_RUN}" == "1" ]]; then
    printf '[DRY RUN] %q' "${cmd}"
    for arg in "$@"; do
      printf ' %q' "$arg"
    done
    printf '\n'
    return 0
  fi

  "${cmd}" "$@"
}

JUST_INSTALL_DIR="${JUST_INSTALL_DIR:-/usr/local/bin}"
JUST_VERSION="${JUST_VERSION:-1.48.1}"


run_as_root() {
  if [[ "${EUID}" -eq 0 ]]; then
    run_cmd "$@"
    return
  fi

  command_exists sudo || die "sudo is required to perform privileged install steps"
  run_cmd sudo "$@"
}

path_writable() {
  local path="$1"

  if [[ -e "${path}" ]]; then
    [[ -w "${path}" ]]
  else
    [[ -w "$(dirname "${path}")" ]]
  fi
}

install_just() {
  local install_dir="${JUST_INSTALL_DIR}"
  local version="${JUST_VERSION}"
  local installer_url="https://just.systems/install.sh"
  local tmp_script

  if ! command_exists curl; then
    command_exists dnf || die "curl is required and dnf is unavailable to install it"
    run_as_root dnf install -y curl
  fi

  info "installing just to ${install_dir}"

  if [[ "${DRY_RUN}" == "1" ]]; then
    if [[ -n "${version}" ]] && [[ "${version}" != "latest" ]]; then
      printf '[DRY RUN] curl --proto ''=https'' --tlsv1.2 -sSf %q -o <tmp> && bash <tmp> -- --to %q --tag %q\n' "${installer_url}" "${install_dir}" "${version}"
    else
      printf '[DRY RUN] curl --proto ''=https'' --tlsv1.2 -sSf %q -o <tmp> && bash <tmp> -- --to %q\n' "${installer_url}" "${install_dir}"
    fi
    return 0
  fi

  if path_writable "${install_dir}"; then
    mkdir -p "${install_dir}"
  else
    run_as_root mkdir -p "${install_dir}"
  fi

  tmp_script="$(mktemp)"
  trap 'rm -f "${tmp_script}"' EXIT

  curl --proto '=https' --tlsv1.2 -sSf "${installer_url}" -o "${tmp_script}"

  if path_writable "${install_dir}"; then
    if [[ -n "${version}" ]] && [[ "${version}" != "latest" ]]; then
      bash "${tmp_script}" -- --to "${install_dir}" --tag "${version}"
    else
      bash "${tmp_script}" -- --to "${install_dir}"
    fi
  else
    if [[ -n "${version}" ]] && [[ "${version}" != "latest" ]]; then
      run_as_root bash "${tmp_script}" -- --to "${install_dir}" --tag "${version}"
    else
      run_as_root bash "${tmp_script}" -- --to "${install_dir}"
    fi
  fi

  if [[ -x "${install_dir}/just" ]]; then
    "${install_dir}/just" --version
  fi

  case ":${PATH}:" in
    *":${install_dir}:"*) ;;
    *) warn "${install_dir} is not on PATH; you may need to add it manually" ;;
  esac

  info "just installation complete"
}

install_just
