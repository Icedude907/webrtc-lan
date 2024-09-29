# I just want to run two commands WITH TERMINAL BEHAVIOUR (colours, loading indicators)
# I can't find an easy way to do this in Rust, JS/TS, or Python3!
# What's the point of scripting languages if you can't quickly cobble something like this together?
# How sad that it must come to this...

# Bring your own JS package manager and replace `pnpm`
# `cargo` is the rust manager/build system

.PHONY: default release

default:
	@echo "Use 'make release' to build release."

# The touch makes sure rust rebuilds the binary assets into the file
release:
	cd webclient && pnpm build
	touch src/webserver.rs
	cargo build --release