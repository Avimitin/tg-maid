{ fetchurl }:
let
  # Fetch only the Japanese faces used by the bot, rather than cloning all CJK fonts.
  base = "https://raw.githubusercontent.com/notofonts/noto-cjk/1c7ca85cb5195a3332e18c2b5cfe196ffb084e72/Sans/OTF/Japanese";
in
{
  bold = fetchurl {
    url = "${base}/NotoSansCJKjp-Black.otf";
    hash = "sha256-zWZoZZSoYaQv51KZCkBvztr7UbOpHWY58w+JSPehC00=";
  };
  light = fetchurl {
    url = "${base}/NotoSansCJKjp-Light.otf";
    hash = "sha256-8puCh0xugM74RXcLNeERzDvf97iyXYpB8ihQgxUuzf4=";
  };
}
