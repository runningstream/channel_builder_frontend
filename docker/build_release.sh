VERSION=`grep version ../backend/Cargo.toml | head -n 1 | cut -f 2 -d '"'`

sudo docker build -t "runningstream/channel_builder_backend:latest" -t "runningstream/channel_builder_backend:$VERSION" -f ../docker/Dockerfile-build ../backend

echo -n "Push version $VERSION? (type yes): "
read DO_PUSH

if [ "$DO_PUSH" = "yes" ]; then
    echo Pushing...
    sudo docker push runningstream/channel_builder_backend:latest
else
    echo Skipping push
fi
