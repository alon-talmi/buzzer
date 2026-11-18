class Buzzer < Formula
  desc "Wait your turn for a shared thing (a test device, an editor), then get buzzed"
  homepage "https://github.com/alon-talmi/buzzer"
  version "0.1.3"
  license "MIT"

  depends_on arch: :arm64
  depends_on :macos

  url "https://github.com/alon-talmi/buzzer/releases/download/v0.1.3/buzzer-v0.1.3-aarch64-apple-darwin.tar.gz"
  sha256 "065e05fbea4513b714e909f37c1656204794fc0399e0a04f601bae374c6fb5d5"

  def install
    bin.install "buzzer"
  end

  test do
    ENV["BUZZER_HOME"] = testpath
    assert_match "YOUR TURN", shell_output("#{bin}/buzzer wait smoke --label t")
  end
end
