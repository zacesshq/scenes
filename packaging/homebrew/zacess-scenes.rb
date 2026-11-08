# Homebrew formula for Zacess Scenes 1.1.0, for Formula/zacess-scenes.rb in the zacesshq/homebrew-tap
# repository. Each SHA256_ placeholder is the sum from the matching .sha256 file attached to the
# v1.1.0 release; packaging/fill-checksums.sh fills them in. The next release changes the version
# in the four URLs and the four sums.
class ZacessScenes < Formula
  desc "Calm animated scenes for the terminal in the ten Zacess accessible themes"
  homepage "https://zacess.com"
  version "1.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/zacesshq/scenes/releases/download/v1.1.0/zacess-scenes-v1.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "SHA256_AARCH64_APPLE_DARWIN"
    end
    on_intel do
      url "https://github.com/zacesshq/scenes/releases/download/v1.1.0/zacess-scenes-v1.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "SHA256_X86_64_APPLE_DARWIN"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/zacesshq/scenes/releases/download/v1.1.0/zacess-scenes-v1.1.0-aarch64-unknown-linux-musl.tar.gz"
      sha256 "SHA256_AARCH64_UNKNOWN_LINUX_MUSL"
    end
    on_intel do
      url "https://github.com/zacesshq/scenes/releases/download/v1.1.0/zacess-scenes-v1.1.0-x86_64-unknown-linux-musl.tar.gz"
      sha256 "SHA256_X86_64_UNKNOWN_LINUX_MUSL"
    end
  end

  def install
    bin.install "zacess-scenes"
    doc.install "README.md", "CHANGELOG.md", "ACCESSIBILITY.md", "NOTICE.md"
  end

  test do
    assert_match "zacess-scenes #{version}", shell_output("#{bin}/zacess-scenes --version")
    system bin/"zacess-scenes", "sea", "--headless", "--frames", "1", "--size", "20x8", "--theme", "deep-dark"
  end
end
