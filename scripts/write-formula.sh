#!/bin/sh
# Usage: write-formula.sh <tag> <folder holding buzzer-<tag>-aarch64-apple-darwin.tar.gz>
set -eu
tag="$1"; dir="$2"; version="${tag#v}"
base="https://github.com/alon-talmi/buzzer/releases/download/$tag"
sha() { shasum -a 256 "$dir/buzzer-$tag-$1.tar.gz" | cut -d' ' -f1; }
cat > Formula/buzzer.rb <<RUBY
class Buzzer < Formula
  desc "Wait your turn for a shared thing (a test device, an editor), then get buzzed"
  homepage "https://github.com/alon-talmi/buzzer"
  version "$version"
  license "MIT"

  depends_on arch: :arm64
  depends_on :macos

  url "$base/buzzer-$tag-aarch64-apple-darwin.tar.gz"
  sha256 "$(sha aarch64-apple-darwin)"

  def install
    bin.install "buzzer"
  end

  test do
    ENV["BUZZER_HOME"] = testpath
    assert_match "YOUR TURN", shell_output("#{bin}/buzzer wait smoke --label t")
  end
end
RUBY
