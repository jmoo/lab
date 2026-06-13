{
  autoPatchelfHook,
  curl,
  fetchurl,
  lib,
  openssl,
  stdenv,
  stdenvNoCC,
}:

# Open-source replacement for Bambu's proprietary network plugin. The upstream
# release ships prebuilt .so per slicer ABI version; we autoPatchelf the set
# matching `abiVersion` against nixpkgs libs so they share the slicer's exact
# glibc/libstdc++ runtime (the mismatch is what abort()s the proprietary blob
# with "free(): invalid size").
#
# Orca dlopens "plugins/libbambu_networking_<network_plugin_version>.so"; the
# upstream installer reports <abiVersion>.99, so we rename the main plugin to
# match and expose that string as passthru.pluginVersion for the conf patch.

let
  abiVersion = "02.03.00"; # Orca 2.3.x ABI; matches OrcaSlicer.conf network_plugin_version prefix
  pluginVersion = "${abiVersion}.99";
in
stdenvNoCC.mkDerivation {
  pname = "open-bamboo-networking";
  version = "1.0.0";

  src = fetchurl {
    hash = "sha256-its+/wr1TABn9LHvrJeWYd3Yxy3eUE+ntF7yPCujAng=";
    url = "https://github.com/ClusterM/open-bamboo-networking/releases/download/v1.0.0/obn-linux-x64.tar.gz";
  };

  nativeBuildInputs = [ autoPatchelfHook ];

  buildInputs = [
    curl
    openssl
    stdenv.cc.cc.lib
  ];

  installPhase = ''
    runHook preInstall

    install -Dm644 lib/v${abiVersion}/libbambu_networking.so \
      "$out/lib/libbambu_networking_${pluginVersion}.so"
    install -Dm644 lib/v${abiVersion}/libBambuSource.so "$out/lib/libBambuSource.so"
    install -Dm644 lib/v${abiVersion}/liblive555.so "$out/lib/liblive555.so"

    runHook postInstall
  '';

  passthru = { inherit abiVersion pluginVersion; };

  meta = {
    description = "Open-source replacement for Bambu Studio's bambu_networking plugin (Orca ${abiVersion} ABI)";
    homepage = "https://github.com/ClusterM/open-bamboo-networking";
    license = lib.licenses.gpl3Plus;
    platforms = [ "x86_64-linux" ];
    sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
  };
}
