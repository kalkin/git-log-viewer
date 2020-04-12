.PHONY: install docs

docs:
	$(MAKE) -C docs

install:
	pip install --user --upgrade .
