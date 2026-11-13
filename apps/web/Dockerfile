# Build context is the repository root; see docker-compose.yml.
#
# syntax=docker/dockerfile:1

FROM node:22-alpine AS builder

# Mirrors the repository layout so Vite's envDir ('../../') resolves inside
# the image the same way it does on a developer's machine.
WORKDIR /repo/apps/web

COPY apps/web/package.json apps/web/package-lock.json ./
RUN npm ci --no-audit --no-fund

COPY apps/web/ ./

# Baked in at build time: adapter-static produces a fully static bundle, so
# this cannot be changed at container start.
ARG PUBLIC_API_BASE_URL=http://localhost:8080
ENV PUBLIC_API_BASE_URL=$PUBLIC_API_BASE_URL

RUN npm run build

FROM nginx:alpine

COPY --from=builder /repo/apps/web/build /usr/share/nginx/html
COPY apps/web/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
