# eyefr/justfile

# Declare the modules and point them to the exact subdirectories
mod eyefr_core "backend/eyefr_core"
mod cam_server "backend/cam_server"
mod db "backend/db"

default:
    @just --list

release-all version:
    @echo "Releasing backend services for version {{ version }}..."
    just eyefr_core release {{ version }}
    just cam_server release {{ version }}
