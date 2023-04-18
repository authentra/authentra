docker run \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 6831:6831/udp \
  -p 5778:5778 \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:1.44