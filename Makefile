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
© smamalig / sjossain - kfs
endef
export HEADER

KERNEL := target/i686-kfs/release/kfs
ISO_DIR := iso
ISO := kfs.iso
ELF:= iso/boot/kfs

header:
	@printf "$(BLUE)$$HEADER$(RESET)\n"

build:
	cargo build --release
	@printf "\n$(BLUE)$$HEADER$(RESET)\n"

iso: build
	mkdir -p $(ISO_DIR)/boot/grub
	cp $(KERNEL) $(ISO_DIR)/boot/kfs
	grub-mkrescue -o $(ISO) $(ISO_DIR)

run: iso
	qemu-system-i386 -cdrom $(ISO)

clean:
	cargo clean
	rm $(ELF)

.PHONY: build iso run clean header
