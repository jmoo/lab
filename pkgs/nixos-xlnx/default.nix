{
  stdenv,
  device-tree-xlnx,
  inputs,
  ...
}:
stdenv.mkDerivation {
  pname = "nixos-xlnx";
  version = "1.0";

  src = inputs.nixos-xlnx;

  patches = [
    ./substitute.patch
  ];
  
  buildPhase = ''
    mkdir -p $out/bin

    for f in ./scripts/*; do
      cp $f $out/bin/
      chmod +x $out/bin/$(basename $f)
      substituteInPlace $out/bin/$(basename $f) \
        --replace-fail "@device-tree-xlnx@" "${device-tree-xlnx}"
    done
  '';
}
