#!/bin/bash -ex
openssl aes-256-cbc -K $encrypted_e850a6789831_key -iv $encrypted_e850a6789831_iv -in index3ds-deploy-key.enc -out index3ds-deploy-key -d
chmod 600 index3ds-deploy-key
eval "$(ssh-agent -s)"
ssh-add index3ds-deploy-key

mkdir deploy
cd deploy
git rev-parse HEAD > ./git-version
mv ../target/release/index3ds ./
mv ../target/release/httpstub ./
mv ../target/deploy ./static

tar -cvzf ../deploy.tar.gz .
cd ..

scp -o "StrictHostKeyChecking no" deploy.tar.gz ubuntu@index3ds.com:/home/ubuntu/index3ds
ssh -o "StrictHostKeyChecking no" ubuntu@index3ds.com 'sudo /home/ubuntu/index3ds/after-deploy.sh'
