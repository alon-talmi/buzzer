class Buzzer < Formula
  desc "Wait your turn for a shared thing (a test device, an editor), then get buzzed"
  homepage "https://github.com/alon-talmi/buzzer"
  version "0.1.2"
  license "MIT"

  depends_on arch: :arm64
  depends_on :macos

  url "https://github.com/alon-talmi/buzzer/releases/download/v0.1.2/buzzer-v0.1.2-aarch64-apple-darwin.tar.gz"
  sha256 "1b288a1f3abab2ea5d8e4973b0057d976ff53ca87c9231d088040f475246fdbc"

  def install
    bin.install "buzzer"
  end

  test do
    ENV["BUZZER_HOME"] = testpath
    assert_match "YOUR TURN", shell_output("#{bin}/buzzer wait smoke --label t")
  end
end
