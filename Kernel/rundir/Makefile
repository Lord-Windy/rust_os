
-include ../../common.mk

.PHONY: run all images

all: run

QEMU_ARGS := -s -no-reboot
ifeq ($(ENABLE_VIDEO),)
	QEMU_ARGS += -nographic
else
	QEMU_ARGS += -serial stdio
endif
QEMU_ARGS += -d int,guest_errors -D qemu_int_log.txt
#QEMU_ARGS += -d int,guest_errors,exec -D qemu_int_log.txt
ifneq ($(QEMU_STALL),)
	QEMU_ARGS += -S
endif
CMDLINE := SYSROOT=/system/Tifflin

ifneq ($(TIFFLIN_INIT),)
	CMDLINE += INIT=$(TIFFLIN_INIT)
endif


ifeq ($(NOTEE),)
 TEE := 2>&1 | tee QemuLog.txt
else
 TEE := 
endif

IMGDIR := DiskImages/$(ARCH)/

#
# --- AMD64 ---
#
ifeq ($(ARCH),amd64)
CMDLINE += SYSDISK=ATA-0p0
QEMU_ARGS += -net nic
QEMU_ARGS += -drive if=ide,index=0,file=$(IMGDIR)hda.img,format=raw
QEMU_ARGS += -cdrom $(IMGDIR)test.iso
QEMU_ARGS += -device ahci,id=ahci
QEMU_ARGS += -drive if=none,id=sata1,file=$(IMGDIR)hdb.img,format=raw -device ide-hd,drive=sata1,bus=ahci.0
QEMU_ARGS += -drive if=none,id=sata2,file=$(IMGDIR)test.iso,format=raw -device ide-cd,drive=sata2,bus=ahci.1
#QEMU_ARGS += -trace file=qemu_trace
#QEMU_OPTS += -trace events=.qemu_trace,file=qemu_trace

MODE ?= pxe
ifeq ($(MODE),iso)
	SCRIPT := RunQemuISO
else
	SCRIPT := RunQemuPXE
endif

run: .qemu_trace images
	./$(SCRIPT) ../bin/kernel-amd64.bin.elf32 "$(CMDLINE)" "$(QEMU_ARGS)" $(TEE)
images: $(IMGDIR)hda.img $(IMGDIR)hdb.img $(IMGDIR)test.iso
#
# --- ARMv7 ---
#
else ifeq ($(ARCH),armv7)
MACHINE_TYPE ?= virt
#MACHINE_TYPE ?= realview-pb-a8

CMDLINE += SYSDISK=virtio0p0
QEMU_ARGS += -device virtio-net-device
QEMU_ARGS += -device virtio-blk-device,drive=hda
QEMU_ARGS += -drive if=none,id=hda,index=0,file=$(IMGDIR)hda.img,format=raw
#QEMU_ARGS += -device pl110
#QEMU_ARGS += -device virtio-gpu-device

run: $(IMGDIR)hda.img
	make -C arm_bootloader PLATFORM=$(MACHINE_TYPE) CMDLINE="$(CMDLINE)"
	qemu-system-arm -machine $(MACHINE_TYPE) $(QEMU_ARGS) -kernel arm_bootloader/loader-$(MACHINE_TYPE).bin -append "$(CMDLINE)" $(TEE)
images: $(IMGDIR)hda.img
endif

$(IMGDIR)test.iso: $(wildcard ../../Usermode/.output/$(ARCH)/bin/*) Makefile
	@mkdir -p $(dir $@)
	@echo "[mkisofs] -o $@"
	@mkisofs -o $@ -r -graft-points -q /Tifflin/bin=../../Usermode/.output/$(ARCH)/bin /Tifflin/shared/images=../../Graphics/.output/shared
$(IMGDIR)hd%_0.img:
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ZERO 1MB $@"
	@# - 1MB of blank space 
	@dd if=/dev/zero of=$@ bs=1M count=1 status=noxfer
$(IMGDIR)hda_1.img: $(wildcard ../../Usermode/.output/$(ARCH)/bin/*) Makefile $(wildcard ../../Graphics/.output/shared/*)
	@mkdir -p $(dir $@)
	@echo "[MkDisk] FAT 32MB $@"
	@# - 32MB FAT? partition on disk 0
	@dd if=/dev/zero of=$@ bs=1M count=32 status=noxfer
	@/sbin/mkfs.vfat $@
	@mmd -i $@ ::/Tifflin
	@mmd -i $@ ::/Tifflin/shared
	@mmd -i $@ ::/Tifflin/shared/images
	@mcopy -s -D o -i $@ ../../Usermode/.output/$(ARCH)/bin ::/Tifflin/bin
	@mcopy -s -D o -i $@ ../../Graphics/.output/shared/* ::/Tifflin/shared/images/
	@echo "Test content" | mcopy -i $@ - ::/1.txt
$(IMGDIR)hda_2.img:
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ext2 16MB $@"
	@# - a 16MB ext2 partition on disk 0
	@#genext2fs -d ../../Usermode/.output/amd64 -b $((16*1024)) $(IMGDIR)hda_2.img 
	@dd if=/dev/zero of=$@ bs=1M count=16 status=noxfer
	@/sbin/mkfs.ext2 -q -F $@
$(IMGDIR)hda.img: Makefile $(IMGDIR)hda_0.img $(IMGDIR)hda_1.img $(IMGDIR)hda_2.img
	@mkdir -p $(dir $@)
	@echo "[MkDisk] mbr $@"
	@# - Commit
	@cat $(IMGDIR)hda_0.img $(IMGDIR)hda_1.img $(IMGDIR)hda_2.img > $(IMGDIR)hda.img
	@echo "$(shell echo $$((1*1024*2)),$$((32*1024*2)),0x83)\n$(shell echo $$((33*1024*2)),+,0x7)" | /sbin/sfdisk --no-reread $(IMGDIR)hda.img -u S -f -q > /dev/null
#	@echo "$(shell echo $$((1*1024*2)),$$((32*1024*2)),0x83)\n$(shell echo $$((33*1024*2)),+,0x7)" | /sbin/sfdisk --no-reread $(IMGDIR)hda.img -C $$((1+32+16)) -H 4 -S 32 -u S -f -q > /dev/null

$(IMGDIR)hdb_1.img: $(wildcard ../../Usermode/.output/$(ARCH)/*) Makefile
	dd if=/dev/zero of=$@ bs=1M count=32
	/sbin/mkfs.ext2 $@
	guestfish -a $@ launch : mount /dev/sda / : mkdir /Tifflin : copy-in ../../Usermode/.output/$(ARCH)/bin /Tifflin/
$(IMGDIR)hdb.img: Makefile $(IMGDIR)hdb_0.img $(IMGDIR)hdb_1.img
	@mkdir -p $(dir $@)
	@echo "[MkDisk] mbr $@"
	@# - Commit
	@cat $(IMGDIR)hdb_0.img $(IMGDIR)hdb_1.img > $@
	@echo "$(shell echo $$((1*1024*2)),+,0x83)" | /sbin/sfdisk --no-reread $@ -u S -f -q > /dev/null

.qemu_trace: Makefile
	echo "" 
	echo "" > $@
	#echo "apic_deliver_irq" >> $@
	#echo "apic_local_deliver" >> $@
	#echo "apic_mem_readl" >> $@
	echo "bdrv_aio_readv" >> $@
	echo "bdrv_aio_writev" >> $@
	echo "bdrv_co_readv" >> $@
	echo "bdrv_co_do_copy_on_readv" >> $@


