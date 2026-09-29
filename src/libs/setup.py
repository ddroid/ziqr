from concurrent.futures import ThreadPoolExecutor

import geocoder
import requests
from eeman.configuration import config, config_path, get_conf

# One session so later surah requests reuse the TLS connection.
_session = requests.Session()

prayer = {"Fajr": "", "Sunrise": "", "Dhuhr": "", "Asr": "", "Maghrib": "", "Isha": ""}
date = None
timezone = None
hijri_date = None


def get_response():
    get_conf()
    method = config["Prayer"]["method"]
    hanafi_school = config["Prayer"]["hanafi_school"]
    location_mode = config["Prayer"]["location_mode"]
    if location_mode == "Automatic":
        get_location_auto()
        get_conf()
    city = config["Prayer"]["city"]
    country = config["Prayer"]["country"]
    method_mode = config["Prayer"]["method_mode"]
    url = "http://api.aladhan.com/v1/timingsByCity"
    params = {"city": city, "country": country, "school": hanafi_school}
    if method_mode == "Manual":
        params["method"] = method

    response = _session.get(url, params=params, timeout=20)
    if response.status_code == 200:
        data = response.json()
        global prayer
        prayer["Fajr"] = data["data"]["timings"]["Fajr"]
        prayer["Sunrise"] = data["data"]["timings"]["Sunrise"]
        prayer["Dhuhr"] = data["data"]["timings"]["Dhuhr"]
        prayer["Asr"] = data["data"]["timings"]["Asr"]
        prayer["Maghrib"] = data["data"]["timings"]["Maghrib"]
        prayer["Isha"] = data["data"]["timings"]["Isha"]

        global date
        global timezone
        global hijri_date
        date = data["data"]["date"]["readable"]
        timezone = data["data"]["meta"]["timezone"]
        hijri_date = "%s %s, %s %s" % (
            data["data"]["date"]["hijri"]["month"]["en"],
            data["data"]["date"]["hijri"]["day"],
            data["data"]["date"]["hijri"]["year"],
            data["data"]["date"]["hijri"]["designation"]["abbreviated"],
        )
    else:
        print(f"Error: {response.status_code}")


def get_location_auto():
    g = geocoder.ip("me")
    set_config("Prayer", "city", g.city)
    set_config("Prayer", "country", g.country)


def set_config(section, option, value):
    get_conf()
    config.set(section, option, value)
    with open(config_path, "w") as file:
        config.write(file)


def _get_json(url, params=None):
    response = _session.get(url, params=params, timeout=20)
    if response.status_code != 200:
        raise RuntimeError("HTTP %s" % response.status_code)
    return response.json()


def get_response_quran_surah_data():
    try:
        return _get_json("https://api.alquran.cloud/v1/surah")
    except Exception as exc:
        print("Error: %s" % exc)
        return None


def fetch_surah_ayah_texts(surah):
    """Arabic and English text for one surah.

    The two requests run together. Returns a list of
    (number_in_surah, english, arabic).
    """
    url_ar = "https://api.alquran.cloud/v1/surah/%s" % surah
    url_en = "https://api.alquran.cloud/v1/surah/%s/en.sahih" % surah
    with ThreadPoolExecutor(max_workers=2) as pool:
        arabic_future = pool.submit(_get_json, url_ar)
        english_future = pool.submit(_get_json, url_en)
        arabic = arabic_future.result()
        english = english_future.result()

    rows = []
    for index, (ar, en) in enumerate(
        zip(arabic["data"]["ayahs"], english["data"]["ayahs"]), start=1
    ):
        rows.append((index, en["text"], ar["text"]))
    return rows


def get_quran_surah_name_english(surah, data):
    return data["data"][surah - 1]["englishName"]


def get_quran_surah_name_arabic(surah, data):
    return data["data"][surah - 1]["name"]


def get_number_of_ayahs(surah, data):
    return data["data"][surah - 1]["numberOfAyahs"]


def get_response_quran_ayah_data_english(surah):
    url = "https://api.alquran.cloud/v1/surah/%s/en.sahih" % surah
    try:
        return _get_json(url)
    except Exception as exc:
        print("Error: %s" % exc)
        return None


def get_response_quran_ayah_data_arabic(surah):
    url = "https://api.alquran.cloud/v1/surah/%s" % surah
    try:
        return _get_json(url)
    except Exception as exc:
        print("Error: %s" % exc)
        return None


def get_quran_ayah_text(data, ayah):
    return data["data"]["ayahs"][ayah - 1]["text"]
