import platform
from os import path, rename, chdir, mkdir, scandir, link
from re import search, findall
from requests import get
from html import unescape
from shutil import copy


def cher_repl(STR):
    for i, j in ("/／", "\\＼", "?？", "|︱", "\"＂", "*＊", "<＜", ">＞"):
        STR = STR.replace(i, j)
    return STR


class bgm_e:
    num_start = 1

    def rename(var1, var2):
        var1 = var2[0]
        var1 = str(path.splitext(var1)[1])
        # 对获取的集数进行排序
        if search(r"(?<=\[)\d+?(?=\]|v2|V2)", str(var2)):
            var2 = sorted(var2, key=lambda j: int(
                findall(r"(?<=\[)\d+?(?=\]|v2|V2)", j)[0]))

      # print(var1,var2)
        bgm_e_num_start = bgm_e.num_start
        for i in range(len(out_file_video_list)):
            if bgm_e_num_start > bgm_e_num_max:
                break
            out_file = str(var2[i])
            bgm_e_name = str(bgm_e_data[bgm_e_num_start-1]['name_cn'])

            bgm_e_name = unescape(bgm_e_name)

            bgm_e_name = cher_repl(bgm_e_name)
            if len(str(bgm_e_num_max)) == 1:
                ep = "ep0"+str(bgm_e_num_start)+" - "
            elif len(str(bgm_e_num_max)) == 2:
                if len(str(bgm_e_num_start)) == 1:
                    ep = "ep0"+str(bgm_e_num_start)+" - "
                else:
                    ep = "ep"+str(bgm_e_num_start)+" - "
            elif len(str(bgm_e_num_max)) == 3:
                if len(str(bgm_e_num_start)) == 1:
                    ep = "ep00"+str(bgm_e_num_start)+" - "
                elif len(str(bgm_e_num_start)) == 2:
                    ep = "ep0"+str(bgm_e_num_start)+" - "
                else:
                    ep = "ep"+str(bgm_e_num_start)+" - "

            rename(out_file, ep+bgm_e_name+var1)
            bgm_e_num_start = bgm_e_num_start + 1


# 选择输入和输出文件夹
in_folder = input("in_folder:")
# 可自定义输出文件夹获取或固定
out_folder = input("out_folder:")
#out_folder = "E:\\动画"
# 调试用路径
#in_folder = "D:\\bgm_rename\\test\\file_test\\in_folder"
#out_folder = "D:\\bgm_rename\\test\\file_test\\out_folder"
# 解决跨平台路径问题
path_xg = "\\"
if platform.system().lower() != 'windows':
    print("linux")
    path_xg = "/"

# 更改工作目录
in_folder = path.abspath(in_folder)
chdir(in_folder)


# 借助bangumi.tv的API搜索番剧信息
bgm_name = input("从bangumi.tv搜索番剧名：")
url = "https://api.bgm.tv/search/subject/" + \
    bgm_name+"?type=2&responseGroup=small"
headers = {
    'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_11_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/50.0.2661.102 Safari/537.36'}
bgm_name_list = get(url, headers=headers).json()

# 获取搜索的番剧中文名列表
bgm_name_list = bgm_name_list["list"]

for i in range(len(bgm_name_list)):
    id = int(bgm_name_list[i]['id'])
    bgm_name_list_cn = bgm_name_list[i]['name_cn']
    bgm_page_url = bgm_name_list[i]['url']
    print(i, " - ", bgm_name_list_cn, " - ", bgm_page_url)

# 选择番剧
i = int(input("请确定番剧是否在目录中，并输入确定的序号："))

# 确定番剧名和ID
bgm_name = str(bgm_name_list[i]['name_cn'])
bgm_id = str(bgm_name_list[i]['id'])

# 获取番剧元数据
url = "https://api.bgm.tv/v0/subjects/"+bgm_id
bgm_meta = get(url, headers=headers).json()
# 确定番剧上映年份
bgm_year = bgm_meta['date'][0:4]

# 创建番剧文件夹
bgm_e_out_folder = bgm_name + " " + "(" + bgm_year + ")"
# 校错，使用特殊字符全角替换半角，避免抛出错误
bgm_e_out_folder = cher_repl(bgm_e_out_folder)

out_folder = out_folder+path_xg+bgm_e_out_folder

if not path.exists(out_folder):
    mkdir(out_folder)

# 创建列表储存文件信息
out_file_video_list = []
out_file_sub_list = []
out_file_sub_sc_list = []
out_file_sub_tc_list = []
# 创建硬链接到番剧文件夹
for item in scandir(in_folder):
    if item.is_dir():
        print("跳过文件夹！！！")
        continue
    s = path.splitext(item.name)[-1][1:]
    video = "avi,mp4,flv,mkv"
    sub = "ass,str"
    out_file = out_folder+path_xg+item.name

    if s in video:
        print("video")
        link(item, out_file)
        out_file_video_list.append(out_file)
        print("创建硬链接：", item.name, " ==> ", out_file)
    elif s in sub:
        print("sub")
        copy(item, out_file)
        if search(r"(?:sc|SC|CHS|chs|gb|GB)", out_file):
            out_file_sub_sc_list.append(out_file)
        elif search(r"(?:tc|TC|CHT|cht|big5|BIG5)", out_file):
            out_file_sub_tc_list.append(out_file)
        else:
            out_file_sub_list.append(out_file)
        print("复制字幕：", item.name, " ==> ", out_file)

# 改变工作目录到番剧文件夹
chdir(out_folder)

# 获取番剧章节列表
url = "https://api.bgm.tv/v0/episodes?subject_id=" + \
    bgm_id+"&type=0&limit=100&offset=0"

bgm_e_meta = get(url, headers=headers).json()


# 获取番剧总集数和每集名称
bgm_e_num_max = int(bgm_e_meta['total'])
bgm_e_data = bgm_e_meta['data']

# 设定从第几集开始获取，默认第1集
bgm_e.num_start = int(input("请输入从第几集开始（默认为第1集）：") or "1")
print("从第 %d 集开始重命名……" % (bgm_e.num_start))

# 视频文件重命名
if out_file_video_list:
    bgm_e.rename("video_file_type", out_file_video_list)

# 字幕文件重命名
if out_file_sub_list:
    bgm_e.rename("sub_file_type", out_file_sub_list)

# sc字幕文件重命名
if out_file_sub_sc_list:
    bgm_e.rename("sub_file_type", out_file_sub_sc_list)

# tc字幕文件重命名
if out_file_sub_tc_list:
    bgm_e.rename("sub_file_type", out_file_sub_tc_list)


print("重命名成功！")
exit()


# episode
# SEASON
