# `visualfft`

![visualfft banner](/images/visualfft-banner.png)

A GUI Tool that can solve and visualize the results of Fast Fourier Transform (FFT) for a given input sequence. Also supports a CLI mode to process CSV files with FFT configurations and output results in the GUI.

![visualfft demo](/images/visualfft-demo.png)

Written in `RUST` with `egui` for GUI.

## Usage:
```bash
visualfft [OPTIONS]
```

## Options
### Input through CSV file
```bash
-c, --csv-file <FILE>
```
Path to CSV file with column: InputSequence

### Input of Sampling interval
```bash
-i, --sampling-interval <DT>
```
Sampling interval dt (required in CLI mode if --sampling-frequency is not provided)

### Input of Sampling frequency
```bash
-f, --sampling-frequency <FS>
```
Sampling frequency fs (required in CLI mode if --sampling-interval is not provided)

### Direction input
```bash
-d, --direction <DIRECTION>
```
Transform direction: forward or inverse `[default: forward]`

### Preview Rows Number
```bash
-p, --preview <ROWS>
```
Number of rows to preview. `[default: 12]`

### Help
```bash
-h, --help
```
Print help (see a summary with '-h')