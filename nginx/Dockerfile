FROM nginx:alpine

# Remove default config
RUN rm /etc/nginx/conf.d/default.conf

# Copy our configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Create directory for SSL certificates (optional)
RUN mkdir -p /etc/nginx/ssl

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost/healthz || exit 1

EXPOSE 80 443

CMD ["nginx", "-g", "daemon off;"]
