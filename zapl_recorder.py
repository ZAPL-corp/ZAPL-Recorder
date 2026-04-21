import sys, cv2, os, json, keyboard, dxcam, time, subprocess, requests, uuid, ctypes, zipfile
import numpy as np
import sounddevice as sd
from PyQt6.QtWidgets import *
from PyQt6.QtCore import QTimer, Qt, QMimeData
from PyQt6.QtGui import QImage, QPixmap, QDrag, QIcon

# Фикс иконки на панели задач
myappid = 'zapl.recorder.pro.v2'
ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(myappid)

RES_DIR = "resources"
if not os.path.exists(RES_DIR): os.makedirs(RES_DIR)
SETTINGS_FILE = os.path.join(RES_DIR, "settings.json")
FFMPEG_PATH = os.path.join(RES_DIR, "ffmpeg.exe")
ICON_PATH = "logo.ico"
DBX_URL = "https://www.dropbox.com"

def get_hwid():
    return str(uuid.getnode())

def download_ffmpeg():
    """Автоматическая загрузка FFmpeg если его нет"""
    if os.path.exists(FFMPEG_PATH): return True
    
    url = "https://www.gyan.dev"
    try:
        print("Загрузка FFmpeg... это займет минуту.")
        r = requests.get(url, stream=True)
        zip_path = os.path.join(RES_DIR, "ffmpeg.zip")
        with open(zip_path, 'wb') as f:
            for chunk in r.iter_content(chunk_size=8192): f.write(chunk)
        
        with zipfile.ZipFile(zip_path, 'r') as zip_ref:
            for file in zip_ref.namelist():
                if file.endswith('ffmpeg.exe'):
                    data = zip_ref.read(file)
                    with open(FFMPEG_PATH, 'wb') as exe_f: exe_f.write(data)
        os.remove(zip_path)
        return True
    except Exception as e:
        print(f"Ошибка загрузки FFmpeg: {e}")
        return False

class ZaplRecorder(QMainWindow):
    def __init__(self):
        super().__init__()
        self.load_settings()
        download_ffmpeg() # Проверка FFmpeg при старте
        
        self.camera = dxcam.create(output_color="BGR", max_buffer_len=5)
        self.recording = False
        self.audio_level = 0
        self.init_ui()
        self.setup_hotkeys()
        if self.settings["mic_enabled"]: self.start_mic_test()

    def load_settings(self):
        default = {"server": "", "key": "", "active_promo": None, "remaining": 0,
                   "video_speed": 30, "mic_enabled": True, "light_theme": False, 
                   "ofs_mode": False, "hwid": None}
        if os.path.exists(SETTINGS_FILE):
            try:
                with open(SETTINGS_FILE, "r") as f: self.settings = {**default, **json.load(f)}
            except: self.settings = default
        else: self.settings = default

    def save_all_settings(self):
        self.settings.update({
            "server": self.server_input.text(), "key": self.key_input.text(),
            "mic_enabled": self.mic_check.isChecked(), "video_speed": self.speed_slider.value(),
            "light_theme": self.theme_check.isChecked(), "ofs_mode": self.ofs_check.isChecked()
        })
        with open(SETTINGS_FILE, "w") as f: json.dump(self.settings, f)
        self.apply_theme(); self.update_window_size()
        self.timer.setInterval(1000 // self.settings["video_speed"])

    def apply_promo(self):
        code = self.promo_input.text().strip(); curr_hwid = get_hwid()
        if self.settings["hwid"] and self.settings["hwid"] != curr_hwid:
            self.status_info.setText("ОШИБКА ПРИВЯЗКИ ПК"); return
        try:
            res = requests.get(DBX_URL, timeout=10)
            if res.status_code == 200:
                for line in res.text.splitlines():
                    if ":" in line:
                        k, v = line.split(":", 1)
                        if k.strip() == code:
                            self.settings.update({"active_promo": k.strip(), "remaining": int(v.strip()), "hwid": curr_hwid})
                            self.status_info.setText(f"ДОБАВЛЕНО: {v.strip()} ВИДЕО"); self.save_all_settings(); return
                self.status_info.setText("КЛЮЧ НЕ НАЙДЕН")
        except: self.status_info.setText("НЕТ ИНТЕРНЕТА")

    def init_ui(self):
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        if os.path.exists(ICON_PATH): self.setWindowIcon(QIcon(ICON_PATH))
        
        self.central_widget = QWidget(); self.setCentralWidget(self.central_widget)
        self.master_layout = QHBoxLayout(self.central_widget); self.master_layout.setContentsMargins(0,0,0,0)
        self.main_container = QStackedWidget(); self.main_container.setFixedSize(400, 700)
        
        self.home_page = QFrame(); self.home_page.setObjectName("MainFrame")
        layout = QVBoxLayout(self.home_page)
        header = QHBoxLayout()
        title = QLabel("  ZAPL RECORDER")
        btn_s = QPushButton("⚙"); btn_s.setFixedWidth(35); btn_s.clicked.connect(lambda: self.main_container.setCurrentIndex(1))
        btn_min = QPushButton("—"); btn_min.setFixedWidth(35); btn_min.clicked.connect(self.showMinimized)
        btn_x = QPushButton("✕"); btn_x.setFixedWidth(35); btn_x.clicked.connect(self.close)
        header.addWidget(title); header.addStretch(); header.addWidget(btn_s); header.addWidget(btn_min); header.addWidget(btn_x)
        layout.addLayout(header)

        self.preview_label = QLabel(); self.preview_label.setObjectName("PreviewLabel")
        self.preview_label.setFixedSize(360, 200); layout.addWidget(self.preview_label, alignment=Qt.AlignmentFlag.AlignCenter)

        self.server_input = QLineEdit(); self.server_input.setPlaceholderText("RTMP Server"); self.server_input.setText(self.settings["server"]); layout.addWidget(self.server_input)
        self.key_input = QLineEdit(); self.key_input.setPlaceholderText("Stream Key"); self.key_input.setEchoMode(QLineEdit.EchoMode.Password); self.key_input.setText(self.settings["key"]); layout.addWidget(self.key_input)
        self.mic_check = QCheckBox("ВКЛЮЧИТЬ МИКРОФОН"); self.mic_check.setChecked(self.settings["mic_enabled"]); layout.addWidget(self.mic_check)

        p_box = QHBoxLayout()
        self.promo_input = QLineEdit(); self.promo_input.setPlaceholderText("Ключ активации")
        btn_ok = QPushButton("ОК"); btn_ok.clicked.connect(self.apply_promo); p_box.addWidget(self.promo_input); p_box.addWidget(btn_ok); layout.addLayout(p_box)

        self.status_info = QLabel(f"Лимит: {self.settings['remaining']} | F9 - Старт"); layout.addWidget(self.status_info, alignment=Qt.AlignmentFlag.AlignCenter)
        self.btn_start = QPushButton("ЗАПИСЬ (F9)"); self.btn_stop = QPushButton("СТОП (F10)"); self.btn_stop.setEnabled(False)
        self.btn_start.clicked.connect(self.start_recording); self.btn_stop.clicked.connect(self.stop_recording)
        layout.addWidget(self.btn_start); layout.addWidget(self.btn_stop)

        # Settings Page
        self.settings_page = QFrame(); self.settings_page.setObjectName("MainFrame")
        s_lay = QVBoxLayout(self.settings_page)
        self.speed_label = QLabel(f"FPS: {self.settings['video_speed']}"); s_lay.addWidget(self.speed_label)
        self.speed_slider = QSlider(Qt.Orientation.Horizontal); self.speed_slider.setRange(10, 60); self.speed_slider.setValue(self.settings["video_speed"])
        self.speed_slider.valueChanged.connect(lambda v: self.speed_label.setText(f"FPS: {v}")); s_lay.addWidget(self.speed_slider)
        self.theme_check = QCheckBox("Светлая тема"); self.theme_check.setChecked(self.settings["light_theme"]); s_lay.addWidget(self.theme_check)
        self.ofs_check = QCheckBox("OFS Mode (Studio)"); self.ofs_check.setChecked(self.settings["ofs_mode"]); s_lay.addWidget(self.ofs_check)
        btn_save = QPushButton("СОХРАНИТЬ"); btn_save.clicked.connect(self.save_all_settings); s_lay.addStretch(); s_lay.addWidget(btn_save)

        self.main_container.addWidget(self.home_page); self.main_container.addWidget(self.settings_page)
        self.ofs_panel = QWidget(); self.ofs_lay = QHBoxLayout(self.ofs_panel)
        self.mic_bar = QProgressBar(); self.mic_bar.setOrientation(Qt.Orientation.Vertical); self.mic_bar.setFixedWidth(20)
        self.ofs_lay.addWidget(self.mic_bar) 
        
        self.master_layout.addWidget(self.main_container); self.master_layout.addWidget(self.ofs_panel)
        self.apply_theme(); self.update_window_size()
        
        self.timer = QTimer(); self.timer.timeout.connect(self.process_frame)
        self.timer.start(1000 // self.settings["video_speed"])

    def apply_theme(self):
        is_l = self.theme_check.isChecked()
        bg = "#ffffff" if is_l else "#0b1a10"; text = "#1e3d2a" if is_l else "#2ecc71"
        self.setStyleSheet(f"QWidget {{ color: {text}; font-weight: bold; font-family: 'Segoe UI'; }} #MainFrame {{ background: {bg}; border: 2px solid #2ecc71; border-radius: 25px; }} QPushButton {{ background: #1e3d2a; color: white; border-radius: 8px; padding: 8px; }} #PreviewLabel {{ background: #000; border-radius: 15px; }} QProgressBar::chunk {{ background: #2ecc71; }}")

    def update_window_size(self):
        self.ofs_panel.setVisible(self.settings["ofs_mode"])
        self.setFixedSize(850 if self.settings["ofs_mode"] else 400, 700)

    def process_frame(self):
        frame = self.camera.grab()
        if frame is not None:
            preview_frame = frame.copy()
            if self.recording:
                if self.settings["remaining"] <= 0:
                    gh = "https://github.com"
                    ts = cv2.getTextSize(gh, cv2.FONT_HERSHEY_SIMPLEX, 0.6, 2)
                    cv2.putText(frame, gh, ((frame.shape[1]-ts[0][0])//2, 50), cv2.FONT_HERSHEY_SIMPLEX, 0.6, (46, 204, 113), 2)
                    not_act = "ZAPL Recorder not activated"
                    ts2 = cv2.getTextSize(not_act, cv2.FONT_HERSHEY_SIMPLEX, 0.6, 2)
                    cv2.putText(frame, not_act, (frame.shape[1]-ts2[0][0]-20, frame.shape[0]-30), cv2.FONT_HERSHEY_SIMPLEX, 0.6, (0, 0, 255), 2)
                
                if hasattr(self, 'stream_proc') and self.stream_proc.poll() is None:
                    try: 
                        self.stream_proc.stdin.write(frame.tobytes())
                        self.stream_proc.stdin.flush()
                    except: self.stop_recording()

                if int(time.time() * 2) % 2 == 0:
                    cv2.circle(preview_frame, (preview_frame.shape[1]-40, 40), 10, (0,0,255), -1)
            
            self.mic_bar.setValue(min(self.audio_level, 100))
            p = cv2.resize(preview_frame, (360, 200))
            img = QImage(cv2.cvtColor(p, cv2.COLOR_BGR2RGB).data, 360, 200, 360*3, QImage.Format.Format_RGB888)
            self.preview_label.setPixmap(QPixmap.fromImage(img))

    def start_recording(self):
        if self.recording or not os.path.exists(FFMPEG_PATH): return
        rtmp = self.server_input.text().strip(); key = self.key_input.text().strip()
        fps = self.settings["video_speed"]
        w, h = self.camera.width, self.camera.height
        
        # Команда FFmpeg: Видео вход (Pipe)
        cmd = [FFMPEG_PATH, '-y', '-f', 'rawvideo', '-vcodec', 'rawvideo', '-pix_fmt', 'bgr24', '-s', f"{w}x{h}", '-r', str(fps), '-i', '-']
        
        # Добавляем микрофон если включен
        if self.settings["mic_enabled"]: cmd += ['-f', 'dshow', '-i', 'audio=default']
        
        if rtmp and key: # Режим стрима
            cmd += ['-c:v', 'libx264', '-preset', 'ultrafast', '-tune', 'zerolatency']
            if self.settings["mic_enabled"]: cmd += ['-c:a', 'aac', '-map', '0:v', '-map', '1:a']
            cmd += ['-f', 'flv', f"{rtmp}/{key}"]
        else: # Режим записи в файл (ТЕПЕРЬ СО ЗВУКОМ)
            path = os.path.join(os.path.expanduser("~"), "Videos", f"ZAPL_{int(time.time())}.mp4")
            cmd += ['-c:v', 'libx264', '-preset', 'ultrafast']
            if self.settings["mic_enabled"]: cmd += ['-c:a', 'aac', '-map', '0:v', '-map', '1:a']
            cmd += [path]
            
        try:
            self.stream_proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, creationflags=0x08000000)
            self.recording = True
            self.btn_start.setEnabled(False); self.btn_stop.setEnabled(True)
            self.status_info.setText("🔴 ИДЕТ ЗАПИСЬ")
        except: self.status_info.setText("ОШИБКА FFmpeg!")

    def stop_recording(self):
        if not self.recording: return
        self.recording = False
        if hasattr(self, 'stream_proc'):
            try: self.stream_proc.stdin.close(); self.stream_proc.terminate()
            except: pass
            del self.stream_proc
        if self.settings["remaining"] > 0: self.settings["remaining"] -= 1; self.save_all_settings()
        self.btn_start.setEnabled(True); self.btn_stop.setEnabled(False); self.status_info.setText(f"Лимит: {self.settings['remaining']}")

    def setup_hotkeys(self):
        keyboard.add_hotkey('f9', self.start_recording)
        keyboard.add_hotkey('f10', self.stop_recording)
        keyboard.add_hotkey('alt+h', self.showMinimized)

    def start_mic_test(self):
        try:
            def cb(i,f,t,s): self.audio_level = int(np.abs(i).mean() * 1000)
            self.stream = sd.InputStream(callback=cb); self.stream.start()
        except: pass

    def mousePressEvent(self, e): 
        if e.button() == Qt.MouseButton.LeftButton: self.drag_pos = e.globalPosition().toPoint()
    def mouseMoveEvent(self, e):
        if hasattr(self, 'drag_pos') and e.buttons() == Qt.MouseButton.LeftButton:
            delta = e.globalPosition().toPoint() - self.drag_pos
            self.move(self.x() + delta.x(), self.y() + delta.y()); self.drag_pos = e.globalPosition().toPoint()

if __name__ == "__main__":
    if not ctypes.windll.shell32.IsUserAnAdmin():
        ctypes.windll.shell32.ShellExecuteW(None, "runas", sys.executable, " ".join(sys.argv), None, 1)
    else:
        app = QApplication(sys.argv); w = ZaplRecorder(); w.show(); sys.exit(app.exec())
