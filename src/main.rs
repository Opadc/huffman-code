use anyhow::Result;
use clap::{App, Arg};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::ops::Add;

#[derive(Debug, Copy, Clone)]
struct Inner {
    ch: u8,
    weight: usize,
}
impl Inner {
    pub fn new(ch: u8, weight: usize) -> Self {
        Inner { ch, weight }
    }
}
impl Add for Inner {
    type Output = Self;
    //非字符节点，ch为0
    fn add(self, other: Self) -> Self {
        Inner {
            ch: 0,
            weight: self.weight + other.weight,
        }
    }
}
#[derive(Debug, Clone)]
struct HuffmanTree {
    inner: Inner,
    left_child: Option<Box<HuffmanTree>>,
    right_child: Option<Box<HuffmanTree>>,
}

impl HuffmanTree {
    fn new(inner: Inner) -> Self {
        HuffmanTree {
            inner,
            left_child: None,
            right_child: None,
        }
    }
    fn merge_hufftree(tree1: Box<HuffmanTree>, tree2: Box<HuffmanTree>) -> Box<HuffmanTree> {
        let merged_node = HuffmanTree {
            inner: tree1.inner + tree2.inner,
            left_child: Some(tree1),
            right_child: Some(tree2),
        };
        Box::new(merged_node)
    }
}

fn main() -> Result<(), anyhow::Error> {
    let matches = App::new("hfcode")
        .version("1.0")
        .author("Opadc")
        .about("use huffman tree to code/decode test")
        .arg(
            Arg::with_name("code")
                .short("c")
                .value_name("FILE")
                .help("Code the FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("decode")
                .short("d")
                .value_name("FILE")
                .help("Decode the FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .value_name("FILE")
                .help("Output file path, default is \".\\output.txt\"")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("codefile")
                .short("t")
                .value_name("FILE")
                .help("The path to keep code table, default is \".\\code.txt\""),
        )
        .get_matches();

    let code_file = matches.value_of("codefile").unwrap_or("code.txt");
    let output_file = matches.value_of("output").unwrap_or("output.txt");

    //压缩选项
    if let Some(file_name) = matches.value_of("code") {
        let mut file = File::open(file_name)?;
        let mut buff = Vec::new();
        let ori_len = file.read_to_end(&mut buff)?;
        let mut freq: HashMap<u8, usize> = HashMap::new();
        for ch in &buff {
            //使用hashtable 记录字符出现频数
            if freq.contains_key(ch) {
                *freq.get_mut(ch).unwrap() += 1;
            } else {
                freq.insert(*ch, 1);
            }
        }

        let trees = create_huffman_tree(freq);
        let mut codes = HashMap::new(); //使用哈希表保存符号及其编码
        let code = Vec::new();
        generate_huffman_code(code, &mut codes, trees); //获取每个字符对应的码表

        write_code2file(&codes, code_file)?;

        let stream = code_original_file(buff, &codes);
        let compressed_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_file)?;
        let mut buff_write = BufWriter::new(compressed_file);
        let comp_len = buff_write.write(&stream)?;
        println!("压缩比为: {}", comp_len as f32 / ori_len as f32);
    }
    //解码选项
    if let Some(file_name) = matches.value_of("decode") {
        let codes = generate_code_table_from_file(code_file)?;

        let mut file = File::open(file_name)?;
        let mut buff = Vec::new();
        file.read_to_end(&mut buff)?;
        buff = buff.into_iter().map(|d| d2b(d)).flatten().collect();

        let decode = decode(codes, buff);

        let mut output = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(output_file)?;
        output.write(&decode)?;
    }
    Ok(())
}

//递归获取编码， code 是到达当前节点时的路径码（左0，右1),
fn generate_huffman_code(
    mut code: Vec<u8>,
    key_code: &mut HashMap<u8, Vec<u8>>,
    mut root: Box<HuffmanTree>,
) {
    if root.inner.ch != 0 {
        key_code.insert(root.inner.ch, code.clone());
        return;
    }
    if root.left_child.is_some() {
        code.push(0);
        generate_huffman_code(code.clone(), key_code, root.left_child.take().unwrap());
    }
    if root.right_child.is_some() {
        code.pop();
        code.push(1);
        generate_huffman_code(code.clone(), key_code, root.right_child.take().unwrap());
    }
}

//由字符频数表建立huffman tree
fn create_huffman_tree(freq: HashMap<u8, usize>) -> Box<HuffmanTree> {
    let mut trees: Vec<Box<HuffmanTree>> = Vec::new(); //森林
    for (ch, weight) in freq.into_iter() {
        trees.push(Box::new(HuffmanTree::new(Inner::new(ch, weight)))); //建立森林
    }
    //插入空树, (解决只有一种字符或空文件)
    trees.push(Box::new(HuffmanTree::new(Inner::new(0, 0))));
    trees.sort_by_key(|x| 0 - x.inner.weight as isize); //根据权值从大到小排序
                                                        //迭代， 不断取出最小权重的树，合并然后插入
    loop {
        if trees.len() == 1 {
            break;
        }
        if let Some(tree1) = trees.pop() {
            if let Some(tree2) = trees.pop() {
                let tree_merged = HuffmanTree::merge_hufftree(tree1, tree2);
                trees.push(tree_merged);
            }
        }
        //循环 不变式:森林永远是有序的
        trees.sort_by_key(|x| 0 - x.inner.weight as isize);
    }
    trees.pop().unwrap()
}

//将huffman 码表写入文件
fn write_code2file(codes: &HashMap<u8, Vec<u8>>, path: &str) -> Result<(), anyhow::Error> {
    let code_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?; //输出码表至文件
    let mut buff_code = BufWriter::new(code_file);
    for (ch, code) in codes {
        //println!("{} {:?}", ch, code);
        let mut tmp = Vec::new();
        tmp.push(*ch);
        tmp.push(b':');
        code.into_iter().for_each(|x| tmp.push(*x + b'0'));
        tmp.push(b'\n');
        buff_code.write(&tmp)?;
    }
    Ok(())
}

//将源文件的字符转变为编码串再转为字节流
fn code_original_file(buff: Vec<u8>, codes: &HashMap<u8, Vec<u8>>) -> Vec<u8> {
    buff.into_iter()
        .map(|ch| codes.get(&ch).unwrap())
        .flat_map(|x| x.clone())
        .collect::<Vec<u8>>()
        .chunks(8)
        .map(|byte| {
            let mut result: u8 = 0;
            for i in 0..byte.len() {
                result += byte[i] * (2u8.pow(7 - i as u32));
            }
            result
        })
        .collect::<Vec<u8>>()
}

fn generate_code_table_from_file(path: &str) -> Result<HashMap<Vec<u8>, u8>, anyhow::Error> {
    let code_file = File::open(path)?;
    let buff_code = BufReader::new(code_file);
    let mut codes = HashMap::new();

    for line in buff_code.lines() {
        let line = line?;
        let line = line.split(":").collect::<Vec<&str>>();
        println!("{:?}", line);
        let ch: char = line.get(0).unwrap().parse()?;
        let ch = ch as u8;
        let code: Vec<u8> = line
            .get(1)
            .unwrap()
            .chars()
            .map(|x| x.to_digit(2).unwrap())
            .map(|x| x as u8)
            .collect();
        codes.insert(code, ch);
    }
    Ok(codes)
}
//单个十进制树转二进制数组
fn d2b(d: u8) -> Vec<u8> {
    let mut result = Vec::new();
    let mut t = 128;
    for _i in 0..8 {
        if d & t != 0 {
            result.push(1);
        } else {
            result.push(0);
        }
        t = t >> 1;
    }
    println!("{:?}", result);
    result
}

fn decode(codes: HashMap<Vec<u8>, u8>, buff: Vec<u8>) -> Vec<u8> {
    let mut window = Vec::new();
    let mut result = Vec::new();
    for b in buff {
        window.push(b);
        if let Some(ch) = codes.get(&window) {
            result.push(*ch);
            window.clear();
        }
    }
    result
}
