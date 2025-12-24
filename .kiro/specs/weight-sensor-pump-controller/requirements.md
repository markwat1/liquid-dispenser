# 要件書

## 概要

Raspberry Pi上で動作する重量センサーとモーター制御を使用した液体計量装置の制御システム。HX711 ADコンバータを使用した重量センサーで最大100gまでの重量を測定し、2つのリレーを使用してモーターの正転・逆転を制御し、指定重量に達したら自動的にモーターを停止して逆転処理を実行する機能を提供する。

## 用語集

- **Weight_Sensor_System**: HX711 ADコンバータと重量センサーを組み合わせた重量測定システム
- **Motor_Controller**: 正転・逆転が可能なモーター制御システム
- **Relay_A**: モーター正転制御用のリレー
- **Relay_B**: モーター逆転制御用のリレー
- **Control_Button**: システムの開始/停止を制御するボタン
- **Target_Weight**: ユーザーが設定する目標重量（グラム単位）
- **Weight_Stabilization**: 重量測定値が安定するまでの待機処理
- **GPIO_Pin**: Raspberry PiのGeneral Purpose Input/Output端子
- **Cross_Compile**: 手元のPCでRaspberry Pi用のバイナリを作成すること

## 要件

### 要件 1

**ユーザーストーリー:** 操作者として、容器を重量センサーに置いてボタンを押すことで液体の計量を開始したい。これにより、手動での液体計量作業を自動化できる。

#### 受入基準

1. WHEN 操作者がControl_Buttonを押下 THEN Weight_Sensor_Systemは3秒間重量測定を実行する
2. WHEN 3秒間の重量測定が完了し値が安定 THEN その値を基準重量（ゼロ点）として設定する
3. WHEN 基準重量設定が完了 THEN Motor_Controllerは正転動作を開始する
4. WHEN Motor_Controllerが動作中 THEN Weight_Sensor_Systemは継続的に重量を監視する
5. WHEN Weight_Sensor_Systemが目標重量を検知 THEN Motor_Controllerは正転を停止する
6. WHEN 正転停止後 THEN Motor_Controllerは3秒間逆転動作を実行する
7. WHEN 3秒間の逆転動作完了 THEN Motor_Controllerは完全停止して処理を終了する
8. WHEN 操作者がControl_Buttonを再度押下 THEN 動作中のMotor_Controllerは緊急停止する

### 要件 2

**ユーザーストーリー:** システム管理者として、モーター制御用のリレー接続を適切に設定したい。これにより、モーターの正転・逆転・停止を確実に制御できる。

#### 受入基準

1. WHEN Relay_AとRelay_Bの両方をOFF THEN Motor_Controllerは停止状態を維持する
2. WHEN Relay_AをON、Relay_BをOFF THEN Motor_Controllerは正転動作を実行する
3. WHEN Relay_AをOFF、Relay_BをON THEN Motor_Controllerは逆転動作を実行する
4. WHEN Relay_AとRelay_Bの両方をON THEN システムはエラー状態として検知する
5. WHEN GPIO_Pinからリレー制御信号を出力 THEN 対応するリレーが確実に動作する

### 要件 3

**ユーザーストーリー:** システム管理者として、HX711とRaspberry Piの接続を適切に設定したい。これにより、重量センサーからの正確なデータを取得できる。

#### 受入基準

1. WHEN HX711のDT端子をRaspberry PiのGPIO_Pinに接続 THEN Weight_Sensor_Systemはデータを受信する
2. WHEN HX711のSCK端子をRaspberry PiのGPIO_Pinに接続 THEN Weight_Sensor_Systemはクロック信号を送信する
3. WHEN HX711のVCC端子を3.3V電源に接続 THEN Weight_Sensor_Systemは正常に動作する
4. WHEN HX711のGND端子をGNDに接続 THEN Weight_Sensor_Systemは安定した動作を維持する
5. WHEN 重量センサーの配線が完了 THEN Weight_Sensor_Systemは0-100gの範囲で重量を測定する

### 要件 4

**ユーザーストーリー:** システム管理者として、ボタンの接続を適切に設定したい。これにより、ユーザー操作を確実に検知できる。

#### 受入基準

1. WHEN Control_ButtonをRaspberry PiのGPIO_Pinに接続 THEN システムはボタン押下を検知する
2. WHEN Control_Buttonにプルアップ抵抗を設定 THEN システムは安定したボタン状態を検知する
3. WHEN GPIO_Pinの設定が完了 THEN システムは各デバイスと正常に通信する
4. WHEN 電源投入時 THEN すべてのGPIO_Pinは安全な初期状態に設定される
5. WHEN システム起動時 THEN Motor_Controllerは停止状態で初期化される

### 要件 5

**ユーザーストーリー:** 操作者として、重量測定の安定化処理を確実に実行したい。これにより、正確な基準重量設定と目標重量検知を実現できる。

#### 受入基準

1. WHEN Weight_Stabilizationを開始 THEN システムは3秒間継続的に重量を測定する
2. WHEN 3秒間の測定中に重量変動が許容範囲内 THEN 測定値を安定として判定する
3. WHEN 3秒間の測定中に重量変動が許容範囲を超過 THEN 安定化処理を再開する
4. WHEN 安定化処理が完了 THEN その時点の重量値を基準重量として記録する
5. WHEN 基準重量設定後 THEN すべての重量測定値から基準重量を差し引いて表示する

### 要件 6

**ユーザーストーリー:** 開発者として、クロスコンパイルによってRaspberry Pi用の実行ファイルを作成したい。これにより、開発環境とターゲット環境を分離できる。

#### 受入基準

1. WHEN 開発PCでクロスコンパイルを実行 THEN ARM64アーキテクチャ用のバイナリが生成される
2. WHEN 生成されたバイナリをRaspberry Piに転送 THEN Ubuntu環境で実行可能である
3. WHEN バイナリを実行 THEN 依存関係なしに動作する
4. WHEN コンパイル時にターゲット重量を指定 THEN バイナリに目標重量が組み込まれる
5. WHEN バイナリサイズが最適化 THEN 効率的なメモリ使用量で動作する

### 要件 7

**ユーザーストーリー:** 操作者として、重量センサーの値を正確に読み取りたい。これにより、精密な液体計量を実現できる。

#### 受入基準

1. WHEN Weight_Sensor_Systemが重量を測定 THEN 測定値は1g単位の精度を持つ
2. WHEN 重量センサーにノイズが発生 THEN システムは移動平均フィルタで値を安定化する
3. WHEN 重量センサーの校正が必要 THEN システムはゼロ点調整機能を提供する
4. WHEN 測定範囲を超過 THEN システムはエラー状態を報告する
5. WHEN HX711との通信でエラー発生 THEN システムは再試行処理を実行する