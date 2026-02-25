#!/bin/bash

tk="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJOYW1lIjoiYWRtaW4iLCJSb2xlIjoiQWRtaW4iLCJDQ29kZSI6IjYyMzIyIiwiZXhwIjoxNzcxNzgwMjU4LCJpc3MiOiJodHRwOi8vbG9jYWxob3N0OjU4ODY3LyIsImF1ZCI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTg4NjcvIn0.48f2vq8wnyvaMFzLNY8V9v8MuPH0jWIOZndF-hqK8YQ"

#curl -v -k -X POST  -H "Content-Type: application/json" -d '{"username":"admin", "password":"njbs1968"}' https://devsys01.tpassvms.com/TpassPVService/api/token

curl -k -v -H "Authorization: Bearer $tk" https://devsys01.tpassvms.com/TpassPVService/api/clients/searchclient?id=Ma&type=All 




