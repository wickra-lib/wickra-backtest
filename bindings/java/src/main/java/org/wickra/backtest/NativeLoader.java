package org.wickra.backtest;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/**
 * Loads the wickra-backtest C ABI library, in this order: {@code java.library.path}
 * (what {@code -Djava.library.path=<dir>} and the test harness set; an explicit
 * choice always wins), the copy bundled in the jar under
 * {@code /native/<os>-<arch>/} (extracted to a temporary file -- how a Maven
 * Central consumer gets it), then every {@code target/release} or
 * {@code target/debug} found by walking up from the working directory and from
 * this class's own location (a checkout after {@code cargo build}).
 *
 * <p>Internal plumbing; application code uses {@link Backtester} and
 * {@link StreamingBacktest}, which resolve their symbols through
 * {@code SymbolLookup.loaderLookup()} once this has run.
 */
final class NativeLoader {
    private NativeLoader() {
    }

    static void load(String name) {
        List<String> rejected = new ArrayList<>();
        try {
            System.loadLibrary(name);
            return;
        } catch (UnsatisfiedLinkError e) {
            rejected.add("java.library.path (" + e.getMessage() + ")");
        }
        String libFile = System.mapLibraryName(name);
        List<Path> candidates = new ArrayList<>();
        Path bundled = extractBundled(libFile);
        if (bundled != null) {
            candidates.add(bundled);
        }
        candidates.addAll(findInCargoTarget(libFile));
        for (Path candidate : candidates) {
            try {
                System.load(candidate.toAbsolutePath().toString());
                return;
            } catch (UnsatisfiedLinkError e) {
                rejected.add(candidate + " (" + e.getMessage() + ")");
            }
        }
        throw new UnsatisfiedLinkError("wickra-backtest: could not load the native library (" + libFile
                + "). Bundle it under resources/native/" + platformDir()
                + "/, pass -Djava.library.path=<dir>, or build the C ABI with"
                + " `cargo build -p wickra-backtest-c --release`."
                + (rejected.isEmpty() ? "" : " Tried: " + String.join("; ", rejected)));
    }

    private static Path extractBundled(String libFile) {
        String resource = "/native/" + platformDir() + "/" + libFile;
        try (InputStream in = NativeLoader.class.getResourceAsStream(resource)) {
            if (in == null) {
                return null;
            }
            Path tmp = Files.createTempFile("wickra-backtest-", "-" + libFile);
            tmp.toFile().deleteOnExit();
            Files.copy(in, tmp, StandardCopyOption.REPLACE_EXISTING);
            return tmp;
        } catch (IOException e) {
            return null;
        }
    }

    /**
     * Walk up from the working directory and from this class's own location
     * looking for {@code target/<profile>/<library>}, collecting every hit.
     */
    private static List<Path> findInCargoTarget(String libFile) {
        List<Path> found = new ArrayList<>();
        for (Path base : new Path[] {Paths.get(System.getProperty("user.dir", ".")), codeSourceDir()}) {
            Path dir = base;
            for (int i = 0; i < 16 && dir != null; i++) {
                for (String profile : new String[] {"release", "debug"}) {
                    Path candidate = dir.resolve("target").resolve(profile).resolve(libFile);
                    if (Files.isRegularFile(candidate) && !found.contains(candidate)) {
                        found.add(candidate);
                    }
                }
                dir = dir.getParent();
            }
        }
        return found;
    }

    private static Path codeSourceDir() {
        try {
            Path p = Paths.get(NativeLoader.class.getProtectionDomain().getCodeSource().getLocation().toURI());
            return Files.isDirectory(p) ? p : p.getParent();
        } catch (Exception e) {
            return null;
        }
    }

    private static String platformDir() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        String osName = os.contains("win") ? "win" : (os.contains("mac") || os.contains("darwin")) ? "osx" : "linux";
        String arch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);
        String archName = (arch.equals("aarch64") || arch.equals("arm64")) ? "arm64" : "x64";
        return osName + "-" + archName;
    }
}
