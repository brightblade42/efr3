# Start from a smaller, more secure base image
FROM docker.io/golang:alpine as builder

# Set the working directory
WORKDIR /app

# Copy go.mod and go.sum files first to leverage Docker cache
COPY go.mod go.sum ./

# Download necessary Go modules
RUN go mod download

# Copy the source code into the container
COPY . .

# Build the Go app
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o server .

# Use a minimal alpine image for the production stage
FROM alpine:latest  

# Add ca-certificates in case you need HTTPS
RUN apk --no-cache add ca-certificates

WORKDIR /root/

# Copy the pre-built binary file from the previous stage
COPY --from=builder /app/server .

# Expose port (adjust if needed)
EXPOSE 8080
EXPOSE 42069

# Command to run the executable
CMD ["./server"]

