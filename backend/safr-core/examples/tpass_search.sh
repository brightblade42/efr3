#!/bin/bash

tk="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJOYW1lIjoiYWRtaW4iLCJSb2xlIjoiQWRtaW4iLCJDQ29kZSI6IjYyMzIyIiwiZXhwIjoxNzcxNTI3MjM4LCJpc3MiOiJodHRwOi8vbG9jYWxob3N0OjU4ODY3LyIsImF1ZCI6Imh0dHA6Ly9sb2NhbGhvc3Q6NTg4NjcvIn0.2W2HZHyvgIbmhTtdd5dr0kfpuLQ-luqCJwfe9OHgY5M"

curl -k -v -H "Authorization: Bearer $tk" https://devsys01.tpassvms.com/TpassPVService/api/clients/searchclient?id=Ma&type=All 




