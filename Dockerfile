FROM ubuntu:latest
LABEL authors="cedric"

ENTRYPOINT ["top", "-b"]