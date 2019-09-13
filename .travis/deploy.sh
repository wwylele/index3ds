#!/bin/bash -ex
openssl aes-256-cbc -K $encrypted_e850a6789831_key -iv $encrypted_e850a6789831_iv -in index3ds-deploy-key.enc -out index3ds-deploy-key -d
chmod 600 index3ds-deploy-key
eval "$(ssh-agent -s)"
ssh-add index3ds-deploy-key

mkdir deploy
mv target/release/index3ds deploy
mv target/release/httpstub deploy
mv target/deploy deploy/static
git rev-parse HEAD > deploy/static/git-revision

tar -cvzf deploy.tar.gz deploy
chmod 666 deploy.tar.gz

scp -o "StrictHostKeyChecking no" deploy.tar.gz ubuntu@index3ds.com:/home/ubuntu/index3ds
ssh -o "StrictHostKeyChecking no" ubuntu@index3ds.com 'sudo /home/ubuntu/index3ds/after-deploy.sh'
