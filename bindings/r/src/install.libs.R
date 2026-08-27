# Install the compiled package object together with the bundled C ABI library, so
# the installed package is self-contained. On Windows that is wickra_backtest.dll
# (matched by the *.dll glob and loaded via PATH from the libs directory); on
# Linux libwickra_backtest.so (matched by the SHLIB_EXT glob); on macOS
# libwickra_backtest.dylib, added explicitly because R package objects use the .so
# extension there too. The rpath baked in by configure ($ORIGIN / @loader_path)
# resolves it from this directory.
files <- unique(c(Sys.glob(paste0("*", SHLIB_EXT)), Sys.glob("libwickra_backtest.dylib")))
dest <- file.path(R_PACKAGE_DIR, paste0("libs", R_ARCH))
dir.create(dest, recursive = TRUE, showWarnings = FALSE)
file.copy(files, dest, overwrite = TRUE)
if (file.exists("symbols.rds")) {
  file.copy("symbols.rds", dest, overwrite = TRUE)
}
