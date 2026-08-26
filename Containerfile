FROM ubuntu:24.04

RUN apt update -y
RUN apt install -y \
	curl \
	mtools \
	xorriso \
	build-essential \
	qemu-system-i386 \
	grub-pc-bin \
	rustup \
	make

WORKDIR /kfs

RUN rustup default nightly

CMD ["/bin/make", "run"]
