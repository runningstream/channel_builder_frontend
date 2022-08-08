#Running

### Frontend

Deploy to S3 and cloudfront

### Backend

Use ansible to setup the server...  Then:

docker-compose -f production-docker-compose.yml up -d

## Building

### Frontend

Install sass by running `npm install sass` in the frontend directory.  Then run `make` in the frontend directory.

### Backend

Run in the docker folder: `./build_backend_container.sh`

Then push the resulting container to docker hub so you can pull it back down with the `production-docker-compose.yml`.

## Backing Up and Restoring the Database

### Backup

`docker-compose -f production-docker-compose.yml exec db pg_dump roku_channel_builder -U postgres > ~/db_backup`

### Restore

First modify the backup file you're going to restore, add the following to the top:

`set session_replication_role = replica;`

Now copy the restore file into the running DB container:

`docker cp BACKUP_FILE CONTAINER_ID:/tmp/db_backup`

Now login to the running DB container:

`docker-compose -f production-docker-compose.yml exec db 'bash'`

Now restore the backup:

`psql roku_channel_builder -U postgres < /tmp/db_backup`

Remove the backup file from the container:

`rm /tmp/db_backup`

## Testing

### Backend

There are docker compose files setup to simplify testing of the backend.  You'll need to have secrets files in the docker directory: db_password.secret, smtp_server.secret, smtp_port.secret, smtp_username.secret, smtp_password.secret, and email_from.secret.  These are just plaintext - do not commit them into git.

Setup an AWS SMTP credential w/IAM policy that only lets you email to your testing address, and use that in the smtp settings.  AWS has some info about how to set that policy [here](https://docs.aws.amazon.com/ses/latest/dg/control-user-access.html).

Change the testing-docker-compose.yml FRONTEND_LOC to be an IP address you can connect to the server via (your local IP address).

Run `docker-compose -f testing-docker-compose.yml up -d` from the docker directory.

Browse to the FRONTEND_LOC you set in a browser.

There's also a (currently very partial) backend_tester python script.

## Credits

* Icons made by https://www.freepik.com from https://www.flaticon.com
* Background image, salzburg.jpg, by Tom Mrazek: https://www.flickr.com/photos/tommrazek/31119289340
