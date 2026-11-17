{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    pkg-config
    cmake
    qt6.qtbase
    qt6.qtdeclarative
    qt6.qttools
    qt6.qtwayland
    fuse3
    gcc
    protobuf
    dbus
    dbus.lib
    cargo-deny
    openssl
  ];

  RUSTFLAGS = "-D warnings";
  
  PKG_CONFIG_PATH = "${pkgs.qt6.qtbase.dev}/lib/pkgconfig:${pkgs.qt6.qtdeclarative.dev}/lib/pkgconfig:${pkgs.fuse3.dev}/lib/pkgconfig:${pkgs.dbus.dev}/lib/pkgconfig";

  shellHook = ''
    # The keyring backend links libdbus at runtime, so the tests need its path.
    export LD_LIBRARY_PATH="${pkgs.dbus.lib}/lib:${pkgs.qt6.qtbase}/lib:${pkgs.qt6.qtdeclarative}/lib:$LD_LIBRARY_PATH"
    echo "Nidvue Drive dev shell"
    echo "Run: cargo xtask ui"
    echo ""
    # Ensure rustup is available and uses Rust 1.95+
    if command -v rustup &> /dev/null; then
      rustup default stable 2>/dev/null || true
      rustup update stable 2>/dev/null || true
    fi
  '';
}
