FROM node:22.22.2-bookworm-slim AS builder

WORKDIR /app

COPY package.json package-lock.json ./

RUN --mount=type=cache,target=/root/.npm,sharing=locked \
    npm ci

COPY index.html tsconfig.json vite.config.ts ./
COPY src ./src

ARG VITE_REALTIME_APPLICATION_ID
ENV VITE_REALTIME_APPLICATION_ID=${VITE_REALTIME_APPLICATION_ID}

RUN npm run build


FROM nginx:1.29-alpine AS runtime

COPY nginx.conf /etc/nginx/nginx.conf
COPY --from=builder /app/dist /usr/share/nginx/html/chat

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --quiet --output-document=/dev/null http://127.0.0.1:8080/chat/ || exit 1
