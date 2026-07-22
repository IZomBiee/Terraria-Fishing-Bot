import onnx

onnx_model = onnx.load(r"sound_detector.onnx")
onnx.save_model(onnx_model, "../assets/sound_detector.onnx", save_as_external_data=False)