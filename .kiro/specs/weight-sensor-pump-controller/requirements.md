# 要件書

## 概要

Raspberry Pi上で動作する重量センサーと蠕動ポンプを使用した液体計量装置の制御システム。HX711 ADコンバータを使用した重量センサーで最大100gまでの重量を測定し、指定重量に達したら自動的にポンプを停止する機能を提供する。

## 用語集

- **Weight_Sensor_System**: HX711 ADコンバータと重量センサーを組み合わせた重量測定システム
- **Peristaltic_Pump**: 蠕動ポンプ。液体を送り出すためのポンプ
- **Control_Button**: システムの開始/停止を制御するボタン
- **Target_Weight**: ユーザーが設定する目標重量（グラム単位）
- **GPIO_Pin**: Raspberry PiのGeneral Purpose Input/Output端子
- **Cross_Compile**: 手元のPCでRaspberry Pi用のバイナリを作成すること

## 要件

### 要件 1

**ユーザーストーリー:** 操作者として、容器を重量センサーに置いてボタンを押すことで液体の計量を開始したい。これにより、手動での液体計量作業を自動化できる。

#### 受入基準

1. WHEN 操作者がControl_Buttonを押下 THEN Weight_Sensor_Systemは液体注入を開始する
2. WHEN 液体注入が開始 THEN Peristaltic_Pumpは動作を開始する
3. WHEN Weight_Sensor_Systemが目標重量を検知 THEN Peristaltic_Pumpは自動的に停止する
4. WHEN 操作者がControl_Buttonを再度押下 THEN 動作中のPeristaltic_Pumpは緊急停止する
5. WHEN システムが待機状態 THEN Weight_Sensor_Systemは現在の重量を継続的に監視する

### 要件 2

**ユーザーストーリー:** システム管理者として、HX711とRaspberry Piの接続を適切に設定したい。これにより、重量センサーからの正確なデータを取得できる。

#### 受入基準

1. WHEN HX711のDT端子をRaspberry PiのGPIO_Pinに接続 THEN Weight_Sensor_Systemはデータを受信する
2. WHEN HX711のSCK端子をRaspberry PiのGPIO_Pinに接続 THEN Weight_Sensor_Systemはクロック信号を送信する
3. WHEN HX711のVCC端子を3.3V電源に接続 THEN Weight_Sensor_Systemは正常に動作する
4. WHEN HX711のGND端子をGNDに接続 THEN Weight_Sensor_Systemは安定した動作を維持する
5. WHEN 重量センサーの配線が完了 THEN Weight_Sensor_Systemは0-100gの範囲で重量を測定する

### 要件 3

**ユーザーストーリー:** システム管理者として、蠕動ポンプとボタンの接続を適切に設定したい。これにより、ポンプの制御とユーザー操作を実現できる。

#### 受入基準

1. WHEN Peristaltic_PumpをRaspberry PiのGPIO_Pinに接続 THEN システムはポンプのON/OFF制御を実行する
2. WHEN Control_ButtonをRaspberry PiのGPIO_Pinに接続 THEN システムはボタン押下を検知する
3. WHEN Control_Buttonにプルアップ抵抗を設定 THEN システムは安定したボタン状態を検知する
4. WHEN GPIO_Pinの設定が完了 THEN システムは各デバイスと正常に通信する
5. WHEN 電源投入時 THEN すべてのGPIO_Pinは安全な初期状態に設定される

### 要件 4

**ユーザーストーリー:** 開発者として、クロスコンパイルによってRaspberry Pi用の実行ファイルを作成したい。これにより、開発環境とターゲット環境を分離できる。

#### 受入基準

1. WHEN 開発PCでクロスコンパイルを実行 THEN ARM64アーキテクチャ用のバイナリが生成される
2. WHEN 生成されたバイナリをRaspberry Piに転送 THEN Ubuntu環境で実行可能である
3. WHEN バイナリを実行 THEN 依存関係なしに動作する
4. WHEN コンパイル時にターゲット重量を指定 THEN バイナリに目標重量が組み込まれる
5. WHEN バイナリサイズが最適化 THEN 効率的なメモリ使用量で動作する

### 要件 5

**ユーザーストーリー:** 操作者として、重量センサーの値を正確に読み取りたい。これにより、精密な液体計量を実現できる。

#### 受入基準

1. WHEN Weight_Sensor_Systemが重量を測定 THEN 測定値は1g単位の精度を持つ
2. WHEN 重量センサーにノイズが発生 THEN システムは移動平均フィルタで値を安定化する
3. WHEN 重量センサーの校正が必要 THEN システムはゼロ点調整機能を提供する
4. WHEN 測定範囲を超過 THEN システムはエラー状態を報告する
5. WHEN HX711との通信でエラー発生 THEN システムは再試行処理を実行する