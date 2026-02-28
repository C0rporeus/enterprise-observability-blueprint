FROM golang:1.24-alpine AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download

COPY main.go ./
RUN CGO_ENABLED=0 GOOS=linux go build -o /go-service main.go

FROM alpine:latest
WORKDIR /
COPY --from=builder /go-service /go-service

EXPOSE 8080
CMD ["/go-service"]