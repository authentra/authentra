FROM node:20-alpine as base
WORKDIR /app
RUN npm install -g pnpm@8.6.5

FROM base as pre-build

COPY pnpm-lock.yaml ./

RUN pnpm fetch

FROM pre-build as builder

ADD . ./
RUN pnpm install -r --offline

RUN pnpm build -m production

FROM node:20-alpine

WORKDIR /app
COPY --from=builder /app/build /app
COPY package.json .
COPY entry.js .

ENV PORT=3000
EXPOSE 3000
CMD [ "node", "entry.js" ]