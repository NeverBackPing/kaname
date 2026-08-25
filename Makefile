# ascii art
BLUE	=	\033[0;34m
GREEN	=	\033[0;32m
RED		=	\033[31m
RESET	=	\033[0m
YELLOW	=	\033[0;33m

define HEADER
██╗  ██╗ █████╗ ███╗   ██╗ █████╗ ███╗   ███╗███████╗
██║ ██╔╝██╔══██╗████╗  ██║██╔══██╗████╗ ████║██╔════╝
█████╔╝ ███████║██╔██╗ ██║███████║██╔████╔██║█████╗
██╔═██╗ ██╔══██║██║╚██╗██║██╔══██║██║╚██╔╝██║██╔══╝
██║  ██╗██║  ██║██║ ╚████║██║  ██║██║ ╚═╝ ██║███████╗
╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝
 Copyright (C) 2026  smamalig, sjossain
 This program comes with ABSOLUTELY NO WARRANTY
 This is free software, and you are welcome to redistribute
 it under certain conditions; See LICENSE for details.
endef
export HEADER

KERNEL := target/i686-kfs/release/kfs
ISO_DIR := iso
ISO := kfs.iso
ELF:= iso/boot/kfs

build:
	cargo build --release
	@printf "\n$(BLUE)$$HEADER$(RESET)\n"

header:
	@printf "$(BLUE)$$HEADER$(RESET)\n"

iso: build
	mkdir -p $(ISO_DIR)/boot/grub
	cp meta/grub.cfg $(ISO_DIR)/boot/grub/grub.cfg
	cp $(KERNEL) $(ISO_DIR)/boot/kfs
	grub-mkrescue -o $(ISO) $(ISO_DIR)

run: iso
	qemu-system-i386 -cdrom $(ISO)

clean:
	cargo clean
	rm $(ELF) kfs.iso


.PHONY: build iso run clean header
