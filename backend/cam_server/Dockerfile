FROM docker.io/denoland/deno:alpine

EXPOSE 3010
EXPOSE 80
WORKDIR /app

ADD . /app

RUN deno cache http_server.ts

CMD ["run", "--allow-all", "http_server.ts"]
