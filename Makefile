RC         := cargo build --release
MKDIR      := mkdir -p
RM         := rm -f
INSTALLBIN := install -C
#INSTALLMAN := install -m 644

BINPREFIX  := target/release
BIN        := pass
DESTPREFIX := /usr/local
DEST       := bin
#MANDIR     := share/man/man1

build:
	$(RC)

install: $(BINPREFIX)/$(BIN)
	$(MKDIR) $(DESTPREFIX)/$(DEST)
	$(INSTALLBIN) $(BINPREFIX)/$(BIN) $(DESTPREFIX)/$(DEST)/$(BIN)

uninstall:
	$(RM) $(DESTPREFIX)/$(DEST)/$(BIN)

.PHONY: build install uninstall
