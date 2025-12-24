#!/bin/sh

# Ensure the datasources directory exists
mkdir -p /etc/grafana/provisioning/datasources

# Substitute environment variables in the Grafana datasources template using sed
# Write to temp file first, then move to avoid permission issues
sed -e "s/\${PROMETHEUS_PORT:-9090}/${PROMETHEUS_PORT:-9090}/g" \
    -e "s/\${TEMPO_PORT:-3200}/${TEMPO_PORT:-3200}/g" \
    -e "s/\${LOKI_PORT:-3100}/${LOKI_PORT:-3100}/g" \
    -e "s/\${OTEL_COLLECTOR_METRICS_PORT:-8888}/${OTEL_COLLECTOR_METRICS_PORT:-8888}/g" \
    /etc/grafana/provisioning/datasources/datasources.yaml.template > /tmp/datasources.yaml

# Move the generated file to the correct location
mv /tmp/datasources.yaml /etc/grafana/provisioning/datasources/datasources.yaml

# Change ownership to grafana user (UID 472)
chown -R 472:0 /etc/grafana/provisioning/datasources/datasources.yaml

# Start Grafana with the default entrypoint
exec /run.sh
