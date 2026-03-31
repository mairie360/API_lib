#!/bin/bash

echo $MAIRIE360_DOCKER_LOGIN | docker login ghcr.io -u $GITHUB_USERNAME --password-stdin