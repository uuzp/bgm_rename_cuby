from requests import get


bgm_name = input("从bangumi.tv搜索番剧名：")
url = "https://api.bgm.tv/search/subject/" + \
    bgm_name+"?type=2&responseGroup=small"
headers = {
    'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_11_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/50.0.2661.102 Safari/537.36'}
bgm_name_list = get(url, headers=headers).json()
print(bgm_name_list)
bgm_name_list = bgm_name_list["list"]
bgm_name_list_new = []
for i in range(len(bgm_name_list)):
    id = int(bgm_name_list[i]['id'])
    
    bgm_name_title = bgm_name_list[i]['name_cn']
    if bgm_name_title == "":
        bgm_name_title = bgm_name_list[i]['name']
    bgm_name_list_new.append(bgm_name_title)
    bgm_page_url = bgm_name_list[i]['url']
    print(i, " - ", bgm_name_title, " - ", bgm_page_url)
print(bgm_name_list_new)