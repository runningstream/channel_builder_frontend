all: website

.PHONY: website sync_s3_files
website: static_files/app.css

sync_s3_files: website
	aws --profile runningstream s3 sync static_files/ s3://runningstream.cc/

static_files/app.css: static_files/app.scss

%.css:
	npx sass $< $@

