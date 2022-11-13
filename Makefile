install:
	poetry install
	poetry run pre-commit install

compile:
	pyoxidizer build --release
	mkdir -p bin
	cp build/**/release/**/renpyfmt/renpyfmt bin/renpyfmt

pc:
	poetry run pre-commit run -a
