#!/bin/bash -ex

openssl aes-256-cbc -K $encrypted_e850a6789831_key -iv $encrypted_e850a6789831_iv -in index3ds-deploy-key.enc -out target/index3ds-deploy-key -d
chmod 600 target/index3ds-deploy-key
ssh-add target/index3ds-deploy-key
ssh ubuntu@index3ds.com 'bash -s' < /home/ubuntu/index3ds/after-deploy.sh
