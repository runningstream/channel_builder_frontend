## Running

### Frontend

Deploy to S3 and cloudfront

### Backend

Use ansible to setup the server...  Then:

docker-compose -f production-docker-compose.yml up -d

## Building

### Frontend

Install sass by running `npm install sass` in the frontend directory.  Then run `make` in the frontend directory.

### Backend



## Backing Up and Restoring the Database

### Backup

`docker-compose -f production-docker-compose.yaml exec db pg_dump roku_channel_builder -U postgres > ~/db_backup`

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

## Credits

* Icons made by https://www.freepik.com from https://www.flaticon.com
* Background image, salzburg.jpg, by Tom Mrazek: https://www.flickr.com/photos/tommrazek/31119289340
