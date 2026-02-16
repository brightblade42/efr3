#!/bin/env bash

DISTRO_NAME="dev_space"
STARTUP_SCRIPT="run-dev.sh"

is_distro_running() {

  distrobox list | grep "$DISTRO_NAME" | grep "Up" >/dev/null

}

up() {

  if ! is_distro_running; then
    echo "container is running, so we'll rudely shut it down and start again"
    distrobox stop "$DISTRO_NAME"

    sleep 5
  fi

  echo "starting dev container"
  distrobox enter "$DISTRO_NAME" -- "$STARTUP_SCRIPT" &

  sleep 5

  if ! is_distro_running; then
    echo "Development environment is now running in $DISTRO_NAME"
  else
    echo "Failed to start the dev environment. Please check for errors."
  fi

}

down() {

  if ! is_distro_running; then
    distrobox stop "$DISTRO_NAME"

    if ! is_distro_running; then
      echo "Development environment in $DISTRO_NAME is stopped"

    else
      echo "Failed to stop the dev environment. You may nee to stop manuall"
    fi
  else
    echo "dev enviroment $DISTRO_NAME not running."
  fi
}

case $1 in
up)
  up
  ;;
down)
  down
  ;;
*)
  echo "Usage: $0 {up|down}"
  echo "  up   - Start the development environment (stops if already running)"
  echo "  down - Stop the development environment"
  exit 1
  ;;
esac
