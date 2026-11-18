class Buzzer < Formula
  desc "Wait your turn for a shared thing (a test device, an editor), then get buzzed"
  homepage "https://github.com/alon-talmi/buzzer"
  version "0.1.5"
  license "MIT"

  depends_on arch: :arm64
  depends_on :macos

  url "https://github.com/alon-talmi/buzzer/releases/download/v0.1.5/buzzer-v0.1.5-aarch64-apple-darwin.tar.gz"
  sha256 "32594ecfa19843f7eb7ad194fb9089e5637e5072cdfa811e3af2a8bce579de31"

  def install
    bin.install "buzzer"
  end

  test do
    ENV["BUZZER_HOME"] = testpath
    assert_match "YOUR TURN", shell_output("#{bin}/buzzer wait smoke --label t")
  end
end
