import os
import platform
import shutil

from requests import get

#选择输入和输出文件夹
in_folder = input("in_folder:")
print("in_folder:",in_folder)
out_folder = input("out_folder:")
print("out_folder:",out_folder)

#解决跨平台路径问题
path_xg = "\\"
if platform.system().lower() != 'windows':
    print("linux")
    path_xg = "/"

#更改工作目录为选择文件夹的父路径
in_folder = os.path.abspath(in_folder)
print(in_folder)
os.chdir(in_folder)



#借助bangumi.tv的API搜索番剧信息
bgm_name = input("从bangumi.tv搜索番剧名：")
url = "https://api.bgm.tv/search/subject/"+bgm_name+"?type=2&responseGroup=small"
headers = {'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_11_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/50.0.2661.102 Safari/537.36'}
bgm_name_list = get(url,headers=headers).json()

#获取搜索的番剧中文名列表
bgm_name_list=bgm_name_list["list"]
for i in range(len(bgm_name_list)):
    id = int(bgm_name_list[i]['id'])
    bgm_name_list_cn = bgm_name_list[i]['name_cn']
    bgm_page_url = bgm_name_list[i]['url']
    print(i," - ",bgm_name_list_cn," - ",bgm_page_url)

#选择番剧
i = int(input("请确定番剧是否在目录中，并输入确定的序号："))

#确定番剧名和ID
bgm_name = str(bgm_name_list[i]['name_cn'])
bgm_id = str(bgm_name_list[i]['id'])

#获取番剧元数据
url = "https://api.bgm.tv/v0/subjects/"+bgm_id
bgm_meta = get(url,headers=headers)
bgm_meta = bgm_meta.json()
#确定番剧上映年份
bgm_year = bgm_meta['date'][0:4]

#创建番剧文件夹
out_folder_new = bgm_name + " " + "(" + bgm_year + ")"
    #校错，使用全角替换半角斜杠，避免抛出错误
if "/" in out_folder_new:
    print("yes")
    out_folder_new = out_folder_new.replace('/','／')

out_folder = out_folder+path_xg+out_folder_new

if not os.path.exists(out_folder):
    os.mkdir(out_folder)

#创建列表储存文件信息
out_file_video_list = []
out_file_sub_list = []

#创建硬链接到番剧文件夹
for item in os.scandir(in_folder):
    if item.is_dir():
        print("跳过文件夹！！！")
        continue
    s = os.path.splitext(item.name)[-1][1:]
    video = "avi,mp4,flv,mkv"
    sub = "ass,str"
    out_file = out_folder+path_xg+item.name

    if s in video:
        print("video")
        os.link(item,out_file) 
        out_file_video_list.append(out_file)
        print("创建硬链接：",item," ==> ",out_file)
    elif s in sub:
        print("sub")
        shutil.copy(item,out_file )
        out_file_sub_list.append(out_file)
        print("复制字幕：",item," ==> ",out_file)
    else:
        print("no video!")

#改变工作目录到番剧文件夹
os.chdir(out_folder)

#获取番剧章节列表
url = "https://api.bgm.tv/v0/episodes?subject_id="+bgm_id+"&type=0&limit=100&offset=0"

bgm_e_meta = get(url,headers=headers)

#格式化json
bgm_e_meta = bgm_e_meta.json()

#获取番剧总集数和每集名称
bgm_e_num = bgm_e_meta['total']
bgm_e_data = bgm_e_meta['data']


#获取视频文件后缀名
if out_file_video_list:
    video_file_type = out_file_video_list[0]
    video_file_type = str(os.path.splitext(video_file_type)[1])
    #视频文件重命名
    for i in range(len(out_file_video_list)):
        out_file_video = str(out_file_video_list[i])
        bgm_e_name = str(bgm_e_data[i]['name_cn'])
        if "/" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('/','／')
        if "<" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('<','〈')
        if ">" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('>','〉')
        if "\\" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('\\','＼')
        if ":" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace(':','：')
        if "*" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('*','·')
        if "?" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('?','？')
        if "\"" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('\"','〃')
        if "|" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('|','｜')
        if i < 9:
            ep = "ep0"+str(i+1)+" - "
            os.rename(out_file_video,ep+bgm_e_name+video_file_type)
        elif i >= bgm_e_num:
            break
        else:
            ep = "ep"+str(i+1)+" - "
            os.rename(out_file_video,ep+bgm_e_name+video_file_type)


# TODO 试着解决存在两种字幕文件的问题，TC.ASS,SC.ASS
#获取字幕文件后缀名
if out_file_sub_list:
    sub_file_type = out_file_sub_list[0]
    sub_file_type = str(os.path.splitext(sub_file_type)[1])
        #字幕文件重命名
    for i in range(len(out_file_video_list)):
        out_file_sub = str(out_file_sub_list[i])
        bgm_e_name = str(bgm_e_data[i]['name_cn'])
        if "/" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('/','／')
        if "<" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('<','〈')
        if ">" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('>','〉')
        if "\\" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('\\','＼')
        if ":" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace(':','：')
        if "*" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('*','·')
        if "?" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('?','？')
        if "\"" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('\"','〃')
        if "|" in bgm_e_name:
            print("yes")
            bgm_e_name = bgm_e_name.replace('|','｜')
        if i < 9:
            ep = "ep0"+str(i+1)+" - "
            os.rename(out_file_sub,ep+bgm_e_name+sub_file_type)
        elif i >= bgm_e_num:
            break
        else:
            ep = "ep"+str(i+1)+" - "
            os.rename(out_file_sub,ep+bgm_e_name+sub_file_type)


print("重命名成功！")
exit()


#episode 
#SEASON
