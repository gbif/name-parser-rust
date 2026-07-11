// We need to forward routine registration from C to Rust
// to avoid the linker removing the static library.

void R_init_nameparser_extendr(void *dll);
void register_extendr_panic_hook(void);

void R_init_nameparser(void *dll) {
    register_extendr_panic_hook();
    R_init_nameparser_extendr(dll);
}
