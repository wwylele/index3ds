#!/bin/bash -ex

openssl aes-256-cbc -K $encrypted_e850a6789831_key -iv $encrypted_e850a6789831_iv -in index3ds-deploy-key.enc -out index3ds-deploy-key -d
chmod 600 index3ds-deploy-key
ssh-add index3ds-deploy-key
ssh ubuntu@index3ds.com '/home/ubuntu/index3ds/after-deploy.sh'
