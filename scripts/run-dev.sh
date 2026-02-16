#!/bin/bash

#strict mode
set -euo pipefail
IFS=$'\n\t'
#this will run a distrobox  and run all out dev processes, deno, go, rust what have you
#we use mutagen to keep our files in sync on a server and so we'll ssh into the machine to run this script
#then we can contiue to develop locally and our changes will be applied on the server, code will reload or recompile on change.
WORKING_DIR="${PWD}"
FRAPI_PATH="backend/safr-core/fr-api"
CAM_SERVER_PATH="backend/cam-server"
host_ip=192.168.3.225


fr_api_port=3100
camserver_port=3011
pvstream_port=5000
pvdetection_port=5050
pvalerts_port=5051
pvdb_port=5432
frdb_port=5433
pvident_port=8080
pvproc_port=8081
rtsp_webrtc_port=8083
facade_port=443
auth_port=42069



#cam server env vars
export FR_DB="${host_ip}" 
export FR_DB_PORT=${frdb_port}
export FR_DB_USER="admin"
export FR_DB_PWD="admin"
export FR_API="http://${host_ip}:${fr_api_port}"
export CAM_SRV_MIN_MATCH=0.50 
export CAM_SRV_MATCH_EXPIRES=10 
export CAM_SRV_MIN_QUALITY=0.8
export CAM_SRV_MIN_DUPE_MATCH=0.98
export CAM_SRV_LOG_DETECTIONS=false
export CAM_SRV_RETAIN_DETECTION_IMAGES=false
export PV_ALERTS_URL="ws://${host_ip}:${pvalerts_port}" 
export PV_DETECTION_URL="ws://${host_ip}:${pvdetection_port}" 
export PV_STREAM_URL="http://${host_ip}:${pvstream_port}" 
export RTSP_API_URL="http://demo:demo@${host_ip}:${rtsp_webrtc_port}" 
export RTSP_CAM_PROXY_URL="http://${host_ip}:${rtsp_webrtc_port}"
export LISTEN_PORT="${camserver_port}"

#fr api env vars
export SAFR_DB_ADDR=${host_ip}
export SAFR_DB_PORT=${frdb_port}
export FRAPI_PORT=${fr_api_port}
export FR_BACKEND=pv
export CV_URL=""
export PV_IDENT_URL="http://${host_ip}:${pvident_port}/v4"
export PV_PROC_URL="http://${host_ip}:${pvproc_port}/v6"
export MIN_MATCH=0.5
export MATCH_EXPIRES=10
export MIN_QUALITY=0.8
export MIN_DUPE_MATCH=0.90
export TPASS_USER="admin" 
export TPASS_PWD="njbs1968"
export TPASS_ADDR=""
export TPASS_ADDR="https://devsys01.tpassvms.com/TpassPVService/"
export TPASS_URL="https://devsys01.tpassvms.com/TpassPVService/"
export RUST_LOG=info
export USE_TLS=false
export TZ=EST

start_proc() {
  local log_file="${!#}"    #get the last argument, the log file
  local cmd=("${@:1:$#-1}") #get all the args up to but not including the last arg , the log file

  "${cmd[@]}" >>$HOME/$log_file 2>&1 &
  echo "Started: ${cmd[*]} (Log: $log_file)"

}

run_api() {

  cd "${FRAPI_PATH}"
  cargo build --bin fr-api
  start_proc cargo watch -x 'run --bin fr-api' fr_api.log

  cd $WORKING_DIR
}

run_cam_server() {

  cd "${CAM_SERVER_PATH}"
  start_proc deno run --allow-all --watch http_server.ts cam_server.log
  cd $WORKING_DIR
}


cleanup() {
  for port in $fr_api_port $cam_server_port; do  # add all your ports here
      pid=$(lsof -ti:$port)
      if [ -n $pid ]; then
        kill -9 $pid
        if [ $? -eq 0 ]; then 
          echo "killed process $pid on port: $port"
        fi
      fi
  done
  echo "Cleanup complete"
}

trap cleanup EXIT INT TERM

run_all() {

  echo "Starting the dev process.."
  run_api
  run_cam_server
  echo "Running"

  wait

  echo "all processes have exited"
}
run_all
