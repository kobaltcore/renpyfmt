install:
	poetry install
	poetry run pre-commit install

compile:
	pyoxidizer build --release

pc:
	poetry run pre-commit run -a
