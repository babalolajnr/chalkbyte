#!/bin/sh

# Substitute environment variables in the Prometheus config template using sed
# Only the app port needs to be substituted - all other targets use internal container ports
sed "s/\${CHALKBYTE_PORT:-3000}/${CHALKBYTE_PORT:-3000}/g" \
    /etc/prometheus/prometheus.yml.template > /etc/prometheus/prometheus.yml

# Start Prometheus with the generated config
exec /bin/prometheus "$@"
