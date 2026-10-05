import sys
s = "日本語テスト 大沢瑠璃乃 " + sys.argv[1]
sys.stdout.reconfigure(encoding="utf-8")
print(s)