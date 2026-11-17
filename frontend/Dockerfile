# syntax=docker/dockerfile:1

# ---------- build: compile the React app to static files ----------
FROM node:22-alpine AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY . .
# The app calls "/api/..." which nginx proxies to the backend container.
ARG VITE_API_BASE_URL=/api
ENV VITE_API_BASE_URL=${VITE_API_BASE_URL}
RUN npm run build

# ---------- runtime: nginx serves the SPA and reverse-proxies /api ----------
FROM nginx:1.27-alpine AS runtime
# Files in /etc/nginx/templates are rendered with envsubst at container start.
COPY nginx/default.conf.template /etc/nginx/templates/default.conf.template
COPY --from=build /app/dist /usr/share/nginx/html
ENV BACKEND_URL=http://backend:8080
EXPOSE 80
HEALTHCHECK --interval=10s --timeout=3s --retries=5 \
    CMD wget -qO- http://127.0.0.1/healthz || exit 1
