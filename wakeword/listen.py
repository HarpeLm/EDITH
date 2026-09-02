"""Écoute continue du micro et signale « Edith » sur stdout.

Protocole (une ligne par événement, flush immédiat) :
  READY              — le modèle est chargé, on écoute
  WAKE               — le mot d'activation a été entendu
  TRANSCRIPT <texte> — ce qui a été dit après le wake word (peut être vide)
puis le script reprend l'écoute. Traitement 100 % local (Vosk, fr small).

Le wake word est détecté dès les résultats partiels (réactivité), et les
variantes phonétiques couvrent les fautes d'orthographe du modèle Vosk.
"""
import json
import queue
import sys

import sounddevice as sd
from vosk import Model, KaldiRecognizer

MODEL_PATH = "wakeword/vosk-model-small-fr-0.22"
SAMPLE_RATE = 16000
# Variantes orthographiques que le modèle français peut produire pour « Edith ».
WAKE_WORDS = (
    "edith", "édith", "é dith", "adith", "édith", "aidith", "editt",
    "édit", "edit", "eydith", "hey dith", "hey edith", "ès dith",
)

q: queue.Queue[bytes] = queue.Queue()


def callback(indata, _frames, _time, status):
    if status:
        print(f"MIC {status}", file=sys.stderr, flush=True)
    q.put(bytes(indata))


def contains_wake(text: str) -> bool:
    t = text.lower()
    return any(w in t for w in WAKE_WORDS)


def after_wake(text: str) -> str:
    """Ce qui suit la première occurrence du wake word dans la phrase."""
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
            if rec.AcceptWaveform(data):
                text = json.loads(rec.Result()).get("text", "")
                if not woke:
                    if contains_wake(text):
                        emit("WAKE")
                        woke = True
                        empty_results = 0
                        tail = after_wake(text)
                        if tail:
                            emit(f"TRANSCRIPT {tail}")
                            woke = False
                # woke : l'énoncé en cours contient « Edith » vu en partiel ;
                # on renvoie ce qui le suit (et rien s'il n'y a rien).
                elif text:
                    tail = after_wake(text) if contains_wake(text) else text
                    emit(f"TRANSCRIPT {tail}")
                    woke = False
            else:
                # Détection précoce sur le partiel : « Edith » est repéré
                # dès qu'il est prononcé, sans attendre la fin de la phrase.
                if not woke:
                    partial = json.loads(rec.PartialResult()).get("partial", "")
                    if contains_wake(partial):
                        emit("WAKE")
                        woke = True


def emit(line: str) -> None:
    print(line, flush=True)


if __name__ == "__main__":
    main()
