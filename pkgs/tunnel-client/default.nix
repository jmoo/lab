{
  autoPatchelfHook,
  fetchurl,
  lib,
  stdenvNoCC,
  unzip,
}:
let
  releases = {
    aarch64-darwin = {
      hash = "sha256-IkPQJLd6+O5LSrehRHlJ5fyNgrzS1RsXiSfy2p5H7P0=";
      platform = "darwin-arm64";
    };
    aarch64-linux = {
      hash = "sha256-WhnlVsSoc2q3pv3m8FaPbnQR+i3/IRhvwVvDDpuYpgM=";
      platform = "linux-arm64";
    };
    x86_64-linux = {
      hash = "sha256-nulEhZI0Wp3RUX+3l5GrVNJPBRFC65K8vozTVe/8kUI=";
      platform = "linux-amd64";
    };
  };
  release = releases.${stdenvNoCC.hostPlatform.system};
in
stdenvNoCC.mkDerivation (finalAttrs: {
  dontStrip = true;
  installPhase = ''
    runHook preInstall
    install -Dm755 tunnel-client $out/bin/tunnel-client
    install -Dm644 LICENSE $out/share/licenses/tunnel-client/LICENSE
    install -Dm644 NOTICE $out/share/licenses/tunnel-client/NOTICE
    install -Dm644 tunnel-client-v*-licenses.txt $out/share/licenses/tunnel-client/third-party.txt
    runHook postInstall
  '';
  meta = {
    description = "Outbound client for OpenAI Secure MCP Tunnel";
    homepage = "https://github.com/openai/tunnel-client";
    license = lib.licenses.asl20;
    mainProgram = "tunnel-client";
    platforms = builtins.attrNames releases;
  };
  nativeBuildInputs = [ unzip ] ++ lib.optional stdenvNoCC.hostPlatform.isLinux autoPatchelfHook;
  pname = "tunnel-client";
  sourceRoot = ".";
  src = fetchurl {
    inherit (release) hash;
    url = "https://github.com/openai/tunnel-client/releases/download/v${finalAttrs.version}/tunnel-client-v${finalAttrs.version}-${release.platform}.zip";
  };
  version = "0.0.19";
})
