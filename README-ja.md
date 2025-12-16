# RP2040/RP2350 USB-UART ブリッジ

Raspberry Pi Pico (RP2040) および Pico 2 (RP2350) 向けの、USB CDC-ACM と UART 間のブリッジ実装です。Rust で書かれた高品質なリファレンス実装です。

[English README](README.md)

## 特徴

- **デュアルコアアーキテクチャ**: Core0 が USB 処理、Core1 が UART 処理
- **ロックフリー通信**: 高性能な SPSC FIFO によるコア間通信
- **マルチボード対応**: RP2040 と RP2350 を単一コードベースでサポート
- **USB CDC-ACM**: 標準シリアルポートインターフェース（カスタムドライバ不要）
- **高スループット**: 115200 ボーの UART で最小レイテンシ
- **安全な Rust**: 最小限の unsafe コードと明確な安全性ドキュメント

## ハードウェア要件

- **Raspberry Pi Pico (RP2040)** または **Raspberry Pi Pico 2 (RP2350)**
- 電源・データ通信用 USB ケーブル
- GPIO0 (TX) と GPIO1 (RX) に接続する UART デバイス

## ソフトウェア要件

### Rust のインストール

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 必要なツールのインストール

```bash
# ARMターゲットを追加
rustup target add thumbv6m-none-eabi      # RP2040用
rustup target add thumbv8m.main-none-eabihf  # RP2350用

# ツールのインストール
cargo install flip-link
cargo install elf2uf2-rs --locked
```

## ビルド

### Raspberry Pi Pico (RP2040) 向け

```bash
cargo rp2040-build
# またはリリース最適化
cargo rp2040-build --release
```

### Raspberry Pi Pico 2 (RP2350) 向け

```bash
cargo rp2350-build
# またはリリース最適化
cargo rp2350-build --release
```

## 書き込み

### 方法 1: BOOTSEL モードを使用（推奨）

1. Pico の BOOTSEL ボタンを押し続ける
2. USB ケーブルをコンピュータに接続
3. BOOTSEL ボタンを離す（Pico が USB ドライブとしてマウントされる）
4. ファームウェアを書き込む：

```bash
# RP2040の場合
cargo rp2040-run
# または
cargo rp2040-run --release

# RP2350の場合
cargo rp2350-run
# または
cargo rp2350-run --release
```

デバイスは自動的に再起動して実行を開始します。

### 方法 2: デバッグプローブを使用（オプション）

デバッグプローブ（例：Raspberry Pi Debug Probe）がある場合：

1. `.cargo/config.toml`の probe-rs ランナーのコメントを外す
2. デバッグプローブを接続
3. 実行: `cargo run --features rp2040` (または`rp2350`)

## 使用方法

### 配線

```
Pico GPIO0 (TX) ──→ ターゲットデバイス RX
Pico GPIO1 (RX) ←── ターゲットデバイス TX
Pico GND ────────── ターゲットデバイス GND
```

### 接続

書き込み後、Pico は USB シリアルポートとして認識されます：

- **Linux**: `/dev/ttyACM0` (または類似)
- **macOS**: `/dev/tty.usbmodemXXXX`
- **Windows**: `COMX`

### 例: screen を使用

```bash
# Linux/macOS
screen /dev/ttyACM0 115200

# 終了: Ctrl+A → K → Y
```

### 例: minicom を使用

```bash
minicom -D /dev/ttyACM0 -b 115200
```

### 例: Python を使用

```python
import serial

ser = serial.Serial('/dev/ttyACM0', 115200)
ser.write(b'Hello UART\n')
response = ser.read(100)
print(response)
ser.close()
```

## プロジェクト構造

```
picoterm-rs/
├── .cargo/
│   └── config.toml        # ビルド設定とエイリアス
├── src/
│   ├── main.rs            # メインエントリポイントとコアロジック
│   ├── uart_core1.rs      # Core1でのUART処理
│   ├── usb_serial.rs      # USBシリアル抽象化
│   └── board/             # ボード固有実装
│       ├── mod.rs         # ボード選択
│       ├── bsp.rs         # HAL再エクスポート
│       ├── rp2040/
│       │   ├── mod.rs     # RP2040固有設定
│       │   └── usb.rs     # RP2040 USB実装
│       └── rp2350/
│           ├── mod.rs     # RP2350固有設定
│           └── usb.rs     # RP2350 USB実装
├── memory.x               # リンカスクリプト
├── Cargo.toml             # プロジェクト依存関係
└── README.md              # 英語版README
```

## アーキテクチャ

### コア割り当て

- **Core0**: USB デバイス処理と USB CDC-ACM プロトコル
- **Core1**: UART 通信（115200 ボー、8N1）

### データフロー

```
PC ←─USB─→ Core0 ←─FIFO─→ Core1 ←─UART─→ 外部デバイス
```

### コア間通信

ロックフリー SPSC（Single Producer Single Consumer）FIFO により、高性能で安全なデータ転送を実現：

- `CDC_TO_UART_QUEUE`: USB → UART（16KB バッファ）
- `UART_TO_CDC_QUEUE`: UART → USB（16KB バッファ）

## 設定

`src/main.rs`を編集してカスタマイズ：

```rust
const UART_BAUD_RATE: u32 = 115_200;  // UARTボーレート
const FIFO_BUFFER_SIZE: usize = 16384; // 各方向のバッファサイズ
```

UART ピンは`src/board/rp2040/mod.rs`（または`rp2350/mod.rs`）で設定：

- GPIO0: UART TX
- GPIO1: UART RX
- GPIO25: LED インジケータ

## LED インジケータ

オンボード LED で動作状態を表示：

- **点灯**: アクティブな USB 通信中
- **点滅**: 最近の通信（10ms ウィンドウ）
- **消灯**: 通信なし

## トラブルシューティング

### USB シリアルポートとして認識されない

1. USB ケーブルがデータ対応か確認（充電専用でないこと）
2. ファームウェアが正しく書き込まれたか確認
3. 別の USB ポートを試す
4. デバイスマネージャー/dmesg でエラーを確認

### データが送受信されない

1. UART 配線を確認（TX ↔ RX、共通グラウンド）
2. ボーレートがターゲットデバイスと一致するか確認
3. ターゲットデバイスの電源を確認
4. ループバックテスト（GPIO0 と GPIO1 を接続）

### ビルドエラー

```bash
# クリーンして再ビルド
cargo clean
cargo rp2040-build
```

### "USB already initialized" パニック

初期化のバグを示しています。issue として報告してください。

## 開発

### テスト実行

```bash
# clippyでコードチェック
cargo clippy --target thumbv6m-none-eabi --features rp2040 --no-default-features
cargo clippy --target thumbv8m.main-none-eabihf --features rp2350 --no-default-features

# コードフォーマット
cargo fmt
```

### 機能追加

アーキテクチャは簡単に拡張できます：

1. `src/board/<board_name>/`にボード固有コードを追加
2. `src/board/mod.rs`を更新して新しいボードを含める
3. `Cargo.toml`にフィーチャーフラグを追加
4. `.cargo/config.toml`にビルドエイリアスを追加

## ライセンス

MIT

## コントリビューション

コントリビューションを歓迎します！以下をお願いします：

1. 既存のコードスタイルに従う（`cargo fmt`を実行）
2. 必要に応じてテストを追加
3. ドキュメントを更新
4. RP2040 と RP2350 両方のビルドが通ることを確認

## 参考資料

- [RP2040 データシート](https://datasheets.raspberrypi.com/rp2040/rp2040-datasheet.pdf)
- [RP2350 データシート](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- [Raspberry Pi Pico SDK](https://github.com/raspberrypi/pico-sdk)
- [usb-device Crate](https://docs.rs/usb-device/)
- [rp2040-hal](https://docs.rs/rp2040-hal/)
- [rp235x-hal](https://docs.rs/rp235x-hal/)

## 謝辞

このプロジェクトは、RP2040/RP2350 プラットフォームのための安全で効率的な組み込み Rust パターンを示しています。
