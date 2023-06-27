#!/usr/bin/env sh
echo "Building backend..."
docker build . -t authentra-backend
cd frontend/
echo "Building frontend..."
docker build . -t authentra-frontend
