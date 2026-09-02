"""Écoute continue du micro et signale « Edith » sur stdout.

Protocole (une ligne par événement, flush immédiat) :
  READY              — le modèle est chargé, on écoute
  WAKE               — le mot d'activation a été entendu
  TRANSCRIPT <texte> — ce qui a été dit après le wake word (peut être vide)
puis le script reprend l'écoute. Traitement 100 % local (Vosk, fr small).
"""
import json
import queue
import sys

import sounddevice as sd
from vosk import Model, KaldiRecognizer

MODEL_PATH = "wakeword/vosk-model-small-fr-0.22"
SAMPLE_RATE = 16000
WAKE_WORDS = ("edith", "édith", "é dith", "adith", "édit", "edit")

q: queue.Queue[bytes] = queue.Queue()


def callback(indata, _frames, _time, status):
    if status:
        print(f"MIC {status}", file=sys.stderr, flush=True)
    q.put(bytes(indata))


def contains_wake(text: str) -> bool:
    t = text.lower()
    return any(w in t for w in WAKE_WORDS)


def after_wake(text: str) -> str:
    t = text.lower()
    for w in WAKE_WORDS:
        i = t.find(w)
        if i >= 0:
            return text[i + len(w):].strip(" ,.!?-")
    return ""


def main() -> None:
    model = Model(MODEL_PATH)
    rec = KaldiRecognizer(model, SAMPLE_RATE)

    emit("READY")
    woke = False
    empty_results = 0

    with sd.RawInputStream(
        samplerate=SAMPLE_RATE, blocksize=4000, dtype="int16",
        channels=1, callback=callback,
    ):
        while True:
            data = q.get()
            if not rec.AcceptWaveform(data):
                continue
            text = json.loads(rec.Result()).get("text", "")
            if not woke:
                if contains_wake(text):
                    emit("WAKE")
                    woke = True
                    empty_results = 0
                    # La commande est parfois dans la même phrase que le wake word.
                    tail = after_wake(text)
                    if tail:
                        emit(f"TRANSCRIPT {tail}")
                        woke = False
            else:
                if text and not contains_wake(text):
                    emit(f"TRANSCRIPT {text}")
                    woke = False
                else:
                    empty_results += 1
                    if empty_results >= 3:  # rien dit de recognizable -> on repart
                        emit("TRANSCRIPT ")
                        woke = False


def emit(line: str) -> None:
    print(line, flush=True)


if __name__ == "__main__":
    main()
