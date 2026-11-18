class Buzzer < Formula
  desc "Wait your turn for a shared thing (a test device, an editor), then get buzzed"
  homepage "https://github.com/alon-talmi/buzzer"
  version "0.1.4"
  license "MIT"

  depends_on arch: :arm64
  depends_on :macos

  url "https://github.com/alon-talmi/buzzer/releases/download/v0.1.4/buzzer-v0.1.4-aarch64-apple-darwin.tar.gz"
  sha256 "16a71e7a165919268da5d5a9c892c747e134255c4f6c8bea41c8fe8e6dcb652e"

  def install
    bin.install "buzzer"
  end

  test do
    ENV["BUZZER_HOME"] = testpath
    assert_match "YOUR TURN", shell_output("#{bin}/buzzer wait smoke --label t")
  end
end
