let
  pkgs = import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/refs/heads/nixos-26.05.tar.gz") { };

  # tauri-driver isn't in nixpkgs — build it from crates.io. First-time
  # evaluation compiles the crate (~30s) and caches it in the Nix store;
  # subsequent nix-shell invocations reuse the cached derivation. Used by
  # the e2e tests under tests/e2e/ to bridge WebdriverIO to WebKitGTK.
  tauri-driver = pkgs.rustPlatform.buildRustPackage rec {
    pname = "tauri-driver";
    version = "0.1.4";
    src = pkgs.fetchCrate {
      inherit pname version;
      hash = "sha256-B9bFFNZZITQvb8CeLgGHNKBaCkj1RvDUh3H6PpXUslk=";
    };
    cargoHash = "sha256-E1BKj2fm5mCoYZRByTs6iStQ8+qjvIyWJ/J71jhENvo=";
    doCheck = false;
  };
in
pkgs.callPackage (
  {
    mkShell,
    lib,
    pkg-config,
    cargo,
    claude-code,
    mistral-vibe,
    rustc,
    rustfmt,
    clippy,
    rust-analyzer,
    nodejs_22,
    sqlite,
    gh,
    openssl,
    glib,
    gtk3,
    cairo,
    gdk-pixbuf,
    pango,
    atk,
    harfbuzz,
    librsvg,
    webkitgtk_4_1,
    libsoup_3,
    libayatana-appindicator,
    xvfb-run,
    gst_all_1,
  }:
  mkShell {
    strictDeps = true;

    nativeBuildInputs = [
      pkg-config
      cargo
      rustc
      rustfmt
      clippy
      rust-analyzer
      nodejs_22
      sqlite
      gh
      claude-code
      mistral-vibe
      tauri-driver
      # Headless X server wrapper so the e2e suite can drive the WebView
      # without a real display: `xvfb-run -a npm run test:e2e`. Mirrors
      # what e2e-nightly.yml does in CI.
      xvfb-run
    ];

    buildInputs = [
      openssl
      glib
      gtk3
      cairo
      gdk-pixbuf
      pango
      atk
      harfbuzz
      librsvg
      webkitgtk_4_1
      libsoup_3
      libayatana-appindicator
      # WebKitGTK loads its media pipeline through GStreamer; without the
      # base/good plugins it logs "GStreamer element appsink not found".
      gst_all_1.gstreamer
      gst_all_1.gst-plugins-base
      gst_all_1.gst-plugins-good
    ];

    shellHook = ''
      export LD_LIBRARY_PATH="${
        lib.makeLibraryPath [
          webkitgtk_4_1
          libsoup_3
          gtk3
          glib
          libayatana-appindicator
        ]
      }:$LD_LIBRARY_PATH"

      # Tauri 2 on some Wayland setups needs DMABuf disabled.
      export WEBKIT_DISABLE_DMABUF_RENDERER=1

      # Let WebKitGTK find the GStreamer plugins added above.
      export GST_PLUGIN_SYSTEM_PATH_1_0="${
        lib.makeSearchPath "lib/gstreamer-1.0" [
          gst_all_1.gstreamer
          gst_all_1.gst-plugins-base
          gst_all_1.gst-plugins-good
        ]
      }"

      # `webkitgtk_4_1` is a buildInput (not nativeBuildInput), so its
      # $out/bin isn't on PATH by default. Add it explicitly so
      # `tauri-driver` can spawn `WebKitWebDriver` during e2e tests.
      # See tests/e2e/README.md.
      export PATH="${webkitgtk_4_1}/bin:$PATH"

      # Dev-only: run with per-save .db snapshots before every overwrite.
      # See `src-tauri/crates/db/Cargo.toml` for what the feature does and
      # its disk-cost expectations. Forwarded to db/save-snapshots via the
      # `save-snapshots` feature on the openfootmanager crate.
      alias ofm-dev-snap='npm run tauri -- dev -f save-snapshots'
    '';
  }
) { }
