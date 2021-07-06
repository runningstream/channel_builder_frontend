all: website

.PHONY: website
website: static_files/app.css

static_files/app.css: static_files/app.scss

%.css:
	npx sass $< $@
