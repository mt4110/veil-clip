# 変更履歴 / Changelog

日本語を先に記載し、各変更に英語を併記します。
Japanese entries come first, followed by English.

## 未リリース / Unreleased

- 2026-10-07：利用場面と現行方式の不一致、追加開発を正当化する別用途が得られなかったことから開発・公開準備を凍結。日英の判断記録と再開条件を追加し、未コミットの検討資料を保持。
  Freeze development/release preparation on 2026-10-07 because the current approach did not fit the reported workflow and no alternative use case justified further work. Preserve investigation documents and add bilingual decision/reopening criteria.

- 監視CLIの公開準備M0〜M2を定義し、旧実装マイルストーンを履歴として保持。通知の価値・品質・導入の完了条件と早期停止条件を日英で追加。PR #1のCI成功・マージを反映し、上書きを予定するように見える冒頭説明を修正。実装・実機操作・公開・定期実行は追加しない。
  Define release-readiness M0–M2 for the observation CLI while preserving earlier implementation milestones. Add bilingual value, quality, delivery and early-stop criteria. Reflect PR #1 CI success/merge and remove introductory replacement expectations. No feature implementation, device operations, publication or scheduling added.

- CIでrustfmtが未導入だった失敗を修正。mise導入後、検証前に固定ツールチェーンのrustfmt・clippyを明示的に導入する。
  Fix CI failure caused by missing rustfmt; explicitly install rustfmt and clippy for the pinned toolchain after mise setup and before checks.

- 機能開発を停止し、5作業日の利用場面確認と、完了・未検証・保留を区別するマイルストーン表を日英で追加。
  Pause feature development; add bilingual five-working-day validation and milestones distinguishing complete, unverified, and on-hold work.

- Rust標準Fileロックとrustixによる安全なパス操作を組み合わせたmacOS排他コンポーネントを追加。独立プロセスの競合・正常終了・SIGKILL後の再取得と保存先の基本安全条件を検証。監視CLIへの接続・上書きは含めない。
  Add a macOS lock component using standard File locking and rustix path operations. Verify independent-process contention, normal/SIGKILL release, and basic path safety; no observation-CLI integration or replacement.

- macOS 27.0.1 / arm64で合成ダミーの検知・5秒後の通知・通常文への切り替えによる取り消し・PTYのCtrl+C正常終了を確認。本文なしの検証記録を日英の設計へ追加。コード修正は不要。
  Verify synthetic-data detection, five-second notification, cancellation on an ordinary-text copy, and normal PTY Ctrl+C shutdown on macOS 27.0.1 / arm64. Add content-free bilingual validation records; no code fix needed.
- AKIA・ASIAのJSON値・URL要素、ASCII全域と非ASCIIの境界、IDと境界を復号しない生テキスト判定の試験を追加。検知コードを変更せず、日英の仕様と検証状況を更新。
  Add tests for AKIA/ASIA in JSON values and URL components, all ASCII and selected non-ASCII boundaries, and raw-text scanning without decoding IDs or boundaries. Update bilingual specifications and verification status without changing detector code.
- 排他ロックの方式比較、JSON・URL等の境界定義、本文なしのログ抑制の限界、将来のメモリゼロ化の導入条件を日英の設計に追記。
  Document lock alternatives, JSON/URL boundary definitions, limits of content-free log suppression, and conditions for future memory zeroization in both languages.
- 初期設計、日英のOSS文書、miseによる開発Rust管理を追加。
  Initial design, bilingual OSS documentation, and development Rust management through mise.
- AWSキーID・秘密鍵ヘッダーの検知と、再確認を要求するTTLコアを実装。
  Implement detection of AWS key IDs/private-key headers and a TTL core requiring a final read.
- 内容変更・読取失敗・サイズ超過で期限を取り消し、監視モードでは書込を要求しない。書込失敗後は停止。
  Cancel deadlines on content changes, read failures, or oversized input; never request writes in observation mode; stop after write failure.
- 合成入力とメモリ上のモックによるコア試験、miseのRust検査・依存監査、CI設定を追加。
  Add core tests using synthetic input and in-memory mocks, mise Rust checks/dependency audits, and CI configuration.
- macOSの監視専用CLI、日英のヘルプ・ログ、読取エラー分類・復帰通知・30秒のログ抑制・Ctrl+C停止処理を追加。
  Add a macOS observation-only CLI, bilingual help/logs, read-error categories, recovery notifications, 30-second log suppression, and Ctrl+C shutdown handling.
- `--apply`と不正引数を初期化前に拒否。モックと安全なコマンドのプロセス試験を追加し、CIをUbuntu・macOSの構成へ更新。
  Reject `--apply` and invalid arguments before initialization; add mock and safe-command process tests; configure Ubuntu and macOS CI.
- 上書き・自動起動は未実装。スリープ復帰・実機障害・競合・性能・実用性とリモートCIは未検証。
  Replacement and automatic startup are not implemented; resume, real-device failures/races, performance, practical value, and remote CI remain unverified.
