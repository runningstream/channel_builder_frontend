all: website

.PHONY: website sync_s3_files
website: content/app.css

sync_s3_files: website
	aws --profile runningstream s3 sync content/ s3://runningstream.cc/

content/app.css: content/app.scss

%.css:
	npx sass $< $@

